use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, PriceLiquidity, Side, Token};
use crate::utils::{bucket_price_levels, merge_two_liquidity_distributions};
use crate::Result;
use alloy_primitives::Address;
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{params, Connection};
use serde_json;
use tracing::info;
use std::ops::Add;
use std::str::FromStr;
use std::sync::{Arc, Mutex};


const WETH_TOKEN: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const USDC_TOKEN: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const DAI_TOKEN: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
const USDT_TOKEN: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
const WBTC_TOKEN: &str = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599";
const DEXES: [&str; 2] = ["uniswap_v2", "uniswap_v3"];

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    // Token operations
    fn save_token(&self, token: &Token) -> Result<()>;
    fn get_token(&self, address: Address, chain_id: u64) -> Result<Option<Token>>;

    // Pool operations
    fn save_pool(&self, pool: &Pool) -> Result<()>;
    fn get_pool(&self, address: Address) -> Result<Option<Pool>>;
    fn get_pools_by_dex(&self, dex: &str, chain_id: u64) -> Result<Vec<Pool>>;
    fn get_pools_by_dex_paginated(&self, dex: &str, chain_id: u64, limit: u64, offset: u64) -> Result<Vec<Pool>>;
    fn get_all_pools_paginated(&self, chain_id: u64, limit: u64, offset: u64) -> Result<Vec<Pool>>;
    fn get_pools_by_token(
        &self,
        token0: Address,
        token1: Address,
        chain_id: u64,
    ) -> Result<Option<Pool>>;

    // Liquidity distribution operations
    fn save_liquidity_distribution(&self, distribution: &LiquidityDistribution) -> Result<()>;
    fn get_liquidity_distribution(
        &self,
        token0: Address,
        token1: Address,
        dex: &str,
        chain_id: u64,
    ) -> Result<Option<LiquidityDistribution>>; // TODO: this should return a vector of LiquidityDistribution


}

pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    pub fn new(database_path: &str) -> Result<Self> {
        let conn = Connection::open(database_path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                address TEXT PRIMARY KEY,
                chain_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                symbol TEXT NOT NULL,
                decimals INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS pools (
                address TEXT PRIMARY KEY,
                chain_id INTEGER NOT NULL,
                dex TEXT NOT NULL,
                token0_address TEXT NOT NULL,
                token1_address TEXT NOT NULL,
                fee INTEGER,
                FOREIGN KEY (token0_address) REFERENCES tokens (address),
                FOREIGN KEY (token1_address) REFERENCES tokens (address)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS liquidity_distributions (
                token0_address TEXT NOT NULL,
                token1_address TEXT NOT NULL,
                dex TEXT NOT NULL,
                chain_id INTEGER NOT NULL,
                data TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                PRIMARY KEY (token0_address, token1_address, dex, chain_id),
                FOREIGN KEY (token0_address) REFERENCES tokens (address),
                FOREIGN KEY (token1_address) REFERENCES tokens (address)
            )",
            [],
        )?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Storage for SqliteStorage {
    fn save_token(&self, token: &Token) -> Result<()> {
        let _address_str = token.address.to_string();
        // TODO: Implement
        Ok(())
    }

    /// Retrieves a token by its address and chain ID.
    ///
    /// Returns `Ok(Some(Token))` if the token exists, or `Ok(None)` if not found. Currently unimplemented and always returns `Ok(None)`.
    fn get_token(&self, address: Address, _chain_id: u64) -> Result<Option<Token>> {
        let _address_str = address.to_string();
        // TODO: Implement

        let conn = self.conn.lock().unwrap();

        let mut stmt = conn
            .prepare(
                "SELECT address, chain_id, name, symbol, decimals
             FROM tokens WHERE address = ?1 AND chain_id = ?2",
            )
            .map_err(|e| Error::DatabaseError(format!("prepare get_token: {e}")))?;

        let token_opt = match stmt.query_row(params![_address_str, _chain_id], |row| {
            let addr: String = row.get(0)?;
            Ok(Token {
                address: Address::from_str(&addr)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                chain_id: row.get(1)?,
                name: row.get(2)?,
                symbol: row.get(3)?,
                decimals: row.get(4)?,
            })
        }) {
            Ok(token) => Some(token),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(Error::DatabaseError(format!("query_row get_token: {e}"))),
        };
        Ok(token_opt)
    }

    /// Saves a pool and its associated tokens to the SQLite database within a transaction.
    ///
    /// Inserts or updates both tokens and the pool record atomically. If any operation fails, the transaction is rolled back.
    ///
    /// # Errors
    ///
    /// Returns `Error::DatabaseError` if any database operation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let storage = SqliteStorage::new(":memory:").unwrap();
    /// let token0 = Token::new(...); // fill with valid data
    /// let token1 = Token::new(...);
    /// let pool = Pool::new(..., vec![token0, token1], ...);
    /// storage.save_pool(&pool).unwrap();
    /// ```
    fn save_pool(&self, pool: &Pool) -> std::result::Result<(), Error> {
        use rusqlite::{params, TransactionBehavior};

        // ① Only connect once, then start transaction
        let mut conn = self.conn.lock().unwrap(); // ← add mut
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| Error::DatabaseError(format!("tx start: {e}")))?;

        // ② Insert or replace two tokens first
        for t in &pool.tokens {
            tx.execute(
                "INSERT OR REPLACE INTO tokens
             (address, chain_id, name, symbol, decimals)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    t.address.to_string(),
                    t.chain_id,
                    t.name,
                    t.symbol,
                    t.decimals as u32
                ],
            )
            .map_err(|e| Error::DatabaseError(format!("save_token: {e}")))?;
        }

        // ③ Pool INSERT
        tx.execute(
            "INSERT OR REPLACE INTO pools
         (address, chain_id, dex, token0_address, token1_address, fee)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                pool.address.to_string(),
                pool.chain_id,
                &pool.dex,
                pool.tokens[0].address.to_string(),
                pool.tokens[1].address.to_string(),
                pool.fee as u32 // Save the actual pool's fee value
            ],
        )
        .map_err(|e| Error::DatabaseError(format!("save_pool: {e}")))?;

        // ④ Commit
        tx.commit()
            .map_err(|e| Error::DatabaseError(format!("commit: {e}")))?;

        Ok(())
    }

    /// Retrieves a pool by its address.
    ///
    /// Returns `Ok(Some(Pool))` if a pool with the specified address exists, or `Ok(None)` if not found. Currently unimplemented and always returns `Ok(None)`.
    fn get_pool(&self, address: Address) -> Result<Option<Pool>> {
        let _address_str = address.to_string();
        // TODO: Implement
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT address, chain_id, dex, token0_address, token1_address, fee
             FROM pools WHERE address = ?1",
            )
            .map_err(|e| Error::DatabaseError(format!("prepare: {e}")))?;
        let (address, chain_id, dex, token0_addr, token1_addr, fee) =
            match stmt.query_row(params![_address_str], |row| {
                Ok((
                    row.get::<_, String>(0)?, // address
                    row.get::<_, u64>(1)?,    // chain_id
                    row.get::<_, String>(2)?, // dex
                    row.get::<_, String>(3)?, // token0_address
                    row.get::<_, String>(4)?, // token1_address
                    row.get::<_, u32>(5)?,    // fee
                ))
            }) {
                Ok(r) => r,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
                Err(e) => return Err(Error::DatabaseError(format!("query_row get_pool: {e}"))),
            };


        let mut token_stmt = conn
            .prepare(
                "SELECT address, chain_id, name, symbol, decimals
             FROM tokens
             WHERE address = ?1 AND chain_id = ?2",
            )
            .map_err(|e| Error::DatabaseError(format!("prepare get_token: {e}")))?;

        let token0: Token = token_stmt
            .query_row(params![token0_addr, chain_id], |row| {
                Ok(Token {
                    address: Address::from_str(&row.get::<_, String>(0)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                    chain_id: row.get(1)?,
                    name: row.get(2)?,
                    symbol: row.get(3)?,
                    decimals: row.get(4)?,
                })
            })
            .map_err(|e| Error::DatabaseError(format!("query_row token0: {e}")))?;

        let token1: Token = token_stmt
            .query_row(params![token1_addr, chain_id], |row| {
                Ok(Token {
                    address: Address::from_str(&row.get::<_, String>(0)?)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                    chain_id: row.get(1)?,
                    name: row.get(2)?,
                    symbol: row.get(3)?,
                    decimals: row.get(4)?,
                })
            })
            .map_err(|e| Error::DatabaseError(format!("query_row token1: {e}")))?;


        let default_dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);

        Ok(Some(Pool {
            address: Address::from_str(&address).unwrap(),
            dex,
            chain_id,
            tokens: vec![token0, token1],
            creation_block: 0, // or fetch from DB if available
            creation_timestamp: default_dt,
            last_updated_block: 0,
            last_updated_timestamp: default_dt,
            fee: fee.into(),
        }))
    }

    /// Retrieves all pools for the specified DEX and chain ID.
    ///
    /// Currently unimplemented; always returns an empty vector.
    fn get_pools_by_dex(&self, dex: &str, chain_id: u64) -> Result<Vec<Pool>> {
        let conn = self.conn.lock().unwrap();
        
        // Use a single query with JOINs to get all required data
        let mut stmt = conn
            .prepare("SELECT p.address, p.chain_id, p.dex, p.token0_address, p.token1_address, p.fee,
                            t0.symbol as token0_symbol, t0.name as token0_name, t0.decimals as token0_decimals,
                            t1.symbol as token1_symbol, t1.name as token1_name, t1.decimals as token1_decimals
                     FROM pools p
                     LEFT JOIN tokens t0 ON p.token0_address = t0.address AND p.chain_id = t0.chain_id
                     LEFT JOIN tokens t1 ON p.token1_address = t1.address AND p.chain_id = t1.chain_id
                     WHERE p.dex = ?1 AND p.chain_id = ?2")
            .map_err(|e| Error::DatabaseError(format!("prepare get_pools_by_dex: {e}")))?;
        
        let mut rows = stmt
            .query(params![dex, chain_id])
            .map_err(|e| Error::DatabaseError(format!("query get_pools_by_dex: {e}")))?;
        
        let mut pools = Vec::new();
        
        while let Some(row) = rows
            .next()
            .map_err(|e| Error::DatabaseError(format!("row get_pools_by_dex: {e}")))?
        {
            let address: String = row.get(0)?;
            let chain_id: u64 = row.get(1)?;
            let dex: String = row.get(2)?;
            let token0_addr: String = row.get(3)?;
            let token1_addr: String = row.get(4)?;
            let fee: u32 = row.get(5)?;
            
            // Parse addresses
            let address = Address::from_str(&address)
                .map_err(|e| Error::DatabaseError(format!("parse pool address: {e}")))?;
            let token0_address = Address::from_str(&token0_addr)
                .map_err(|e| Error::DatabaseError(format!("parse token0 address: {e}")))?;
            let token1_address = Address::from_str(&token1_addr)
                .map_err(|e| Error::DatabaseError(format!("parse token1 address: {e}")))?;
            
            // Get token data from JOIN results
            let token0_symbol: Option<String> = row.get(6)?;
            let token0_name: Option<String> = row.get(7)?;
            let token0_decimals: Option<u8> = row.get(8)?;
            let token1_symbol: Option<String> = row.get(9)?;
            let token1_name: Option<String> = row.get(10)?;
            let token1_decimals: Option<u8> = row.get(11)?;
            
            // Skip pools where token info is missing
            if token0_symbol.is_none() || token1_symbol.is_none() {
                continue;
            }
            
            let token0 = Token {
                address: token0_address,
                symbol: token0_symbol.unwrap(),
                name: token0_name.unwrap(),
                decimals: token0_decimals.unwrap(),
                chain_id,
            };
            
            let token1 = Token {
                address: token1_address,
                symbol: token1_symbol.unwrap(),
                name: token1_name.unwrap(),
                decimals: token1_decimals.unwrap(),
                chain_id,
            };
            
            // Create default timestamps (same as get_pool)
            let default_dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);
            
            let pool = Pool {
                address,
                dex,
                chain_id,
                tokens: vec![token0, token1],
                creation_block: 0,
                creation_timestamp: default_dt,
                last_updated_block: 0,
                last_updated_timestamp: default_dt,
                fee: fee.into(),
            };
            
            pools.push(pool);
        }
        
        Ok(pools)
    }

    fn get_pools_by_dex_paginated(&self, dex: &str, chain_id: u64, limit: u64, offset: u64) -> Result<Vec<Pool>> {
        let conn = self.conn.lock().unwrap();
        
        // Use a single query with JOINs to get all required data with pagination
        let mut stmt = conn
            .prepare("SELECT p.address, p.chain_id, p.dex, p.token0_address, p.token1_address, p.fee,
                            t0.symbol as token0_symbol, t0.name as token0_name, t0.decimals as token0_decimals,
                            t1.symbol as token1_symbol, t1.name as token1_name, t1.decimals as token1_decimals
                     FROM pools p
                     LEFT JOIN tokens t0 ON p.token0_address = t0.address AND p.chain_id = t0.chain_id
                     LEFT JOIN tokens t1 ON p.token1_address = t1.address AND p.chain_id = t1.chain_id
                     WHERE p.dex = ?1 AND p.chain_id = ?2
                     ORDER BY p.rowid
                     LIMIT ?3 OFFSET ?4")
            .map_err(|e| Error::DatabaseError(format!("prepare get_pools_by_dex_paginated: {e}")))?;
        
        let mut rows = stmt
            .query(params![dex, chain_id, limit, offset])
            .map_err(|e| Error::DatabaseError(format!("query get_pools_by_dex_paginated: {e}")))?;
        
        let mut pools = Vec::new();
        
        while let Some(row) = rows
            .next()
            .map_err(|e| Error::DatabaseError(format!("row get_pools_by_dex_paginated: {e}")))?
        {
            let address: String = row.get(0)?;
            let chain_id: u64 = row.get(1)?;
            let dex: String = row.get(2)?;
            let token0_addr: String = row.get(3)?;
            let token1_addr: String = row.get(4)?;
            let fee: u32 = row.get(5)?;
            
            // Parse addresses
            let address = Address::from_str(&address)
                .map_err(|e| Error::DatabaseError(format!("parse pool address: {e}")))?;
            let token0_address = Address::from_str(&token0_addr)
                .map_err(|e| Error::DatabaseError(format!("parse token0 address: {e}")))?;
            let token1_address = Address::from_str(&token1_addr)
                .map_err(|e| Error::DatabaseError(format!("parse token1 address: {e}")))?;
            
            // Get token data from JOIN results
            let token0_symbol: Option<String> = row.get(6)?;
            let token0_name: Option<String> = row.get(7)?;
            let token0_decimals: Option<u8> = row.get(8)?;
            let token1_symbol: Option<String> = row.get(9)?;
            let token1_name: Option<String> = row.get(10)?;
            let token1_decimals: Option<u8> = row.get(11)?;
            
            // Skip pools where token info is missing
            if token0_symbol.is_none() || token1_symbol.is_none() {
                continue;
            }
            
            let token0 = Token {
                address: token0_address,
                symbol: token0_symbol.unwrap(),
                name: token0_name.unwrap(),
                decimals: token0_decimals.unwrap(),
                chain_id,
            };
            
            let token1 = Token {
                address: token1_address,
                symbol: token1_symbol.unwrap(),
                name: token1_name.unwrap(),
                decimals: token1_decimals.unwrap(),
                chain_id,
            };
            
            // Create default timestamps (same as get_pool)
            let default_dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);
            
            let pool = Pool {
                address,
                dex,
                chain_id,
                tokens: vec![token0, token1],
                creation_block: 0,
                creation_timestamp: default_dt,
                last_updated_block: 0,
                last_updated_timestamp: default_dt,
                fee: fee.into(),
            };
            
            pools.push(pool);
        }
        
        Ok(pools)
    }

    fn get_all_pools_paginated(&self, chain_id: u64, limit: u64, offset: u64) -> Result<Vec<Pool>> {
        let conn = self.conn.lock().unwrap();
        
        let mut stmt = conn
            .prepare("SELECT p.address, p.chain_id, p.dex, p.token0_address, p.token1_address, p.fee,
                            t0.symbol as token0_symbol, t0.name as token0_name, t0.decimals as token0_decimals,
                            t1.symbol as token1_symbol, t1.name as token1_name, t1.decimals as token1_decimals
                     FROM pools p
                     LEFT JOIN tokens t0 ON p.token0_address = t0.address AND p.chain_id = t0.chain_id
                     LEFT JOIN tokens t1 ON p.token1_address = t1.address AND p.chain_id = t1.chain_id
                     WHERE p.chain_id = ?1 AND p.dex IN ('uniswap_v3', 'uniswap_v2', 'sushiswap')
                     ORDER BY p.rowid DESC
                     LIMIT ?2 OFFSET ?3")
            .map_err(|e| Error::DatabaseError(format!("prepare get_all_pools_paginated: {e}")))?;
        
        let mut rows = stmt
            .query(params![chain_id, limit, offset])
            .map_err(|e| Error::DatabaseError(format!("query get_all_pools_paginated: {e}")))?;
        
        let mut pools = Vec::new();
        
        while let Some(row) = rows
            .next()
            .map_err(|e| Error::DatabaseError(format!("row get_all_pools_paginated: {e}")))?
        {
            let address: String = row.get(0)?;
            let chain_id: u64 = row.get(1)?;
            let dex: String = row.get(2)?;
            let token0_addr: String = row.get(3)?;
            let token1_addr: String = row.get(4)?;
            let fee: u32 = row.get(5)?;
            
            // Parse addresses
            let address = Address::from_str(&address)
                .map_err(|e| Error::DatabaseError(format!("parse pool address: {e}")))?;
            let token0_address = Address::from_str(&token0_addr)
                .map_err(|e| Error::DatabaseError(format!("parse token0 address: {e}")))?;
            let token1_address = Address::from_str(&token1_addr)
                .map_err(|e| Error::DatabaseError(format!("parse token1 address: {e}")))?;
            
            // Get token data from JOIN results
            let token0_symbol: Option<String> = row.get(6)?;
            let token0_name: Option<String> = row.get(7)?;
            let token0_decimals: Option<u8> = row.get(8)?;
            let token1_symbol: Option<String> = row.get(9)?;
            let token1_name: Option<String> = row.get(10)?;
            let token1_decimals: Option<u8> = row.get(11)?;
            
            // Skip pools where token info is missing
            if token0_symbol.is_none() || token1_symbol.is_none() {
                continue;
            }
            
            let token0 = Token {
                address: token0_address,
                symbol: token0_symbol.unwrap(),
                name: token0_name.unwrap(),
                decimals: token0_decimals.unwrap(),
                chain_id,
            };
            
            let token1 = Token {
                address: token1_address,
                symbol: token1_symbol.unwrap(),
                name: token1_name.unwrap(),
                decimals: token1_decimals.unwrap(),
                chain_id,
            };
            
            // Create default timestamps (same as get_pool)
            let default_dt = DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);
            
            let pool = Pool {
                address,
                dex,
                chain_id,
                tokens: vec![token0, token1],
                creation_block: 0,
                creation_timestamp: default_dt,
                last_updated_block: 0,
                last_updated_timestamp: default_dt,
                fee: fee.into(),
            };
            
            pools.push(pool);
        }
        
        Ok(pools)
    }

    /// Retrieves all pools that include the specified token address.
    ///
    /// Currently unimplemented; always returns an empty vector.
    ///
    /// # Parameters
    /// - `token_address`: The address of the token to search for in pools.
    ///
    /// # Returns
    /// A vector of pools containing the specified token address, or an empty vector if none are found.
    ///
    /// # Examples
    ///
    /// ```
    /// let pools = storage.get_pools_by_token(token_address).unwrap();
    /// assert!(pools.is_empty());
    /// ```
    fn get_pools_by_token(
        &self,
        token0: Address,
        token1: Address,
        chain_id: u64,
    ) -> Result<Option<Pool>> {
        let conn = self.conn.lock().unwrap();

        // First try with token0 as token0_address and token1 as token1_address
        let mut stmt = conn
            .prepare(
                "SELECT p.address, p.chain_id, p.dex, p.token0_address, p.token1_address, p.fee
             FROM pools p
             WHERE p.token0_address = ?1 AND p.token1_address = ?2 AND p.chain_id = ?3
             UNION
             SELECT p.address, p.chain_id, p.dex, p.token0_address, p.token1_address, p.fee
             FROM pools p
             WHERE p.token0_address = ?2 AND p.token1_address = ?1 AND p.chain_id = ?3
             LIMIT 1",
            )
            .map_err(|e| Error::DatabaseError(format!("prepare get_pools_by_token: {e}")))?;

        let pool_result = stmt.query_row(
            params![token0.to_string(), token1.to_string(), chain_id],
            |row| {
                let addr: String = row.get(0)?;
                let chain_id: u64 = row.get(1)?;
                let dex: String = row.get(2)?;
                let token0_addr: String = row.get(3)?;
                let token1_addr: String = row.get(4)?;
                let _fee: u32 = row.get(5)?;

                // Get token0 info
                let mut token_stmt = conn
                    .prepare(
                        "SELECT address, chain_id, name, symbol, decimals
                     FROM tokens
                     WHERE address = ? AND chain_id = ?",
                    )
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                let token0 = token_stmt.query_row(params![token0_addr, chain_id], |row| {
                    Ok(Token {
                        address: Address::from_str(&row.get::<_, String>(0)?)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                        chain_id: row.get(1)?,
                        name: row.get(2)?,
                        symbol: row.get(3)?,
                        decimals: row.get(4)?,
                    })
                })?;

                // Get token1 info
                let token1 = token_stmt.query_row(params![token1_addr, chain_id], |row| {
                    Ok(Token {
                        address: Address::from_str(&row.get::<_, String>(0)?)
                            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                        chain_id: row.get(1)?,
                        name: row.get(2)?,
                        symbol: row.get(3)?,
                        decimals: row.get(4)?,
                    })
                })?;

                let default_dt =
                    DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);

                Ok(Pool {
                    address: Address::from_str(&addr)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                    dex,
                    chain_id,
                    tokens: vec![token0, token1],
                    creation_block: 0,
                    creation_timestamp: default_dt,
                    last_updated_block: 0,
                    last_updated_timestamp: default_dt,
                    fee: _fee.into(),
                })
            },
        );

        match pool_result {
            Ok(pool) => Ok(Some(pool)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(format!(
                "get_pools_by_token error: {e}"
            ))),
        }
    }

    /// Saves a liquidity distribution record to the storage.
    ///
    /// Currently unimplemented; calling this method has no effect and always returns success.
    fn save_liquidity_distribution(&self, distribution: &LiquidityDistribution) -> Result<()> {
        use rusqlite::{params, TransactionBehavior};

        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| Error::DatabaseError(format!("tx start: {e}")))?;
        let data = serde_json::to_string(&distribution)
            .map_err(|e| Error::DatabaseError(format!("serialize distribution: {e}")))?;
        tx.execute(
            "INSERT OR REPLACE INTO liquidity_distributions
            (token0_address, token1_address, dex, chain_id, data, timestamp)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                distribution.token0.address.to_string(),
                distribution.token1.address.to_string(),
                distribution.dex,
                distribution.chain_id,
                data,
                distribution.timestamp.timestamp()
            ],
        )
        .map_err(|e| Error::DatabaseError(format!("save_liquidity_distribution: {e}")))?;

        // Commit the transaction
        tx.commit()
            .map_err(|e| Error::DatabaseError(format!("commit: {e}")))?;

        Ok(())
        // let _token0_address_str = distribution.token0.address.to_string();
        // let _token1_address_str = distribution.token1.address.to_string();
        // // TODO: Implement
        // Ok(())
    }

    /// Retrieves the liquidity distribution for a given token pair, DEX, and chain ID.
    ///
    /// Returns `Ok(Some(LiquidityDistribution))` if a matching record exists, or `Ok(None)` if not found. Currently unimplemented and always returns `Ok(None)`.
    fn get_liquidity_distribution(
        &self,
        token0: Address,
        token1: Address,
        dex: &str,
        chain_id: u64,
    ) -> Result<Option<LiquidityDistribution>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT data
                 FROM liquidity_distributions
                 WHERE token0_address = ?
                   AND token1_address = ?
                   AND dex            = ?
                   AND chain_id       = ?
                 ORDER BY timestamp DESC
                 LIMIT 1",
            )
            .map_err(|e| Error::DatabaseError(format!("prepare get_liquidity_distribution: {e}")))?;
        
        let row_res: rusqlite::Result<String> = stmt.query_row(
            params![
                token0.to_string(),
                token1.to_string(),
                dex,
                chain_id,
            ],
            |row| row.get(0),
        );
    
        let json_str = match row_res {
            Ok(s) => s,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => {
                return Err(Error::DatabaseError(format!(
                    "get_liquidity_distribution query error: {e}"
                )))
            }
        };

        let distribution: LiquidityDistribution = serde_json::from_str(&json_str)
            .map_err(|e| Error::DatabaseError(format!("JSON parse error: {e}")))?;

        Ok(Some(distribution))
    }
}
    // get_pools_by_token0 : only input token0 address & query all the pools that have token0 as token0_address
// pub async fn get_pools_by_token0(
//     storage: Arc<dyn Storage>,
//     token0: Address,
//     chain_id: u64,
// ) -> Result<Vec<Pool>> {
//     let conn = self.conn.lock().unwrap();
//         let mut stmt = conn
//             .prepare(
//                 "SELECT data FROM liquidity_distributions 
//              WHERE token0_address = ?1  AND dex = ?2 AND chain_id = ?3
//              ORDER BY timestamp DESC LIMIT 1",
//             )
//             .map_err(|e| {
//                 Error::DatabaseError(format!("prepare get_liquidity_distribution: {e}"))
//             })?;
//     storage.get_pools_by_token(token0, Address::default(), chain_id)
// }   

pub async fn reverse_pair(
    distribution: LiquidityDistribution,
) -> Result<LiquidityDistribution> {
    let LiquidityDistribution {
        token0,
        token1,
        dex,
        chain_id,
        timestamp,
        current_price,
        price_levels,
    } = distribution;

    let reversed_levels: Vec<PriceLiquidity> = price_levels
        .into_iter()
        .map(|pl| PriceLiquidity {
            side: pl.side,  // 필요하다면 Buy<->Sell도 뒤집을 수 있음
            // 가격 구간도 상하한을 뒤집어서 역수로
            lower_price: 1.0 / pl.upper_price,
            upper_price: 1.0 / pl.lower_price,
            // 토큰 유동성도 서로 스왑
            token0_liquidity: pl.token1_liquidity,
            token1_liquidity: pl.token0_liquidity,
            timestamp: pl.timestamp,
        })
        .collect();

    let reversed_price = 1.0 / current_price;

    let reversed_distribution = LiquidityDistribution {
        token0: token1,
        token1: token0,
        dex,
        chain_id,
        timestamp,
        current_price: reversed_price,
        price_levels: reversed_levels,
    };

    Ok(reversed_distribution)
}

pub async fn reverse_current_price_only(
    distribution: LiquidityDistribution,
) -> Result<LiquidityDistribution> {
    Ok(LiquidityDistribution {
        current_price: 1.0 / distribution.current_price,
        ..distribution
    })
}


pub async fn get_current_price(
    storage: Arc<dyn Storage>,
    token0: Address,
    token1: Address,
    dex : &str,
    chain_id: u64,
    )  -> Result<f64> {
    let liquidity_distribution = 
        match storage.get_liquidity_distribution  (token0, token1, dex, chain_id)? {
            Some(distribution) => Some(distribution),
            // If no distribution found for (token0, token1), try (token1, token0)
            None => match storage.get_liquidity_distribution(token1, token0, dex, chain_id)?
            {
                Some(distribution) => Some(reverse_current_price_only(distribution).await?),
                None => return Ok(0.0), // Return 0.0 if no distribution found for both pairs
            },
        };
    if let Some(distribution) = liquidity_distribution {
        return Ok(distribution.current_price)
    }
    Ok(0.0)
}

// TODO: consider a case where token1 is in the place of token0
// TODO: aggregate with equal interval
pub async fn aggregate_liquidity_dexes(
    storage: Arc<dyn Storage>,
    token1: Address,
    chain_id: u64,
) -> Result<LiquidityDistribution> {
    let mut distributions = Vec::new();
    for &dex in &DEXES {
        let dist = aggregate_liquidity_token1(
            storage.clone(),
            token1,
            dex,
            chain_id,
        )
        .await?;
        distributions.push(dist);
    }

    if distributions.is_empty() {
        return Err(Error::DexError("No liquidity distributions found for any DEX".to_string()));
    }

    let mut merged_dist = distributions[0].clone();
    for dist_to_merge in distributions.iter().skip(1) {
        if let Some(newly_merged) = merge_two_liquidity_distributions(&merged_dist, dist_to_merge) {
            merged_dist = newly_merged;
        } else {
            info!("Failed to merge distributions for token {}", token1);
        }
    }
    merged_dist.dex = "aggregated".to_string();

    Ok(merged_dist)
}

pub async fn aggregate_liquidity_token1(
    storage: Arc<dyn Storage>,
    token1: Address,
    dex_for_price_reference : &str,
    chain_id: u64,
) -> Result<LiquidityDistribution>{
    let Token1 = storage.get_token(token1, chain_id)?
        .ok_or(Error::InvalidAddress(token1.to_string()))?;
    let usdc_address = Address::from_str(USDC_TOKEN).unwrap();
    
    let token_constants = [WETH_TOKEN, WBTC_TOKEN, USDT_TOKEN, DAI_TOKEN, USDC_TOKEN];
    let mut token_prices = Vec::new();
    
    for &token_str in &token_constants {
        let mut price = 1.0;
        if token_str != USDT_TOKEN && token_str != DAI_TOKEN && token_str != USDC_TOKEN {
            price = get_current_price(
                storage.clone(), 
                Address::from_str(token_str).unwrap(), 
                usdc_address, 
                dex_for_price_reference, 
                chain_id
            ).await?;
        }

        token_prices.push((Address::from_str(token_str).unwrap(), price));
    }
    
    // get token1 - WETH/USDC/DAI/USDT/WBTC pair price
    let dummy_dist = LiquidityDistribution {
        token0: Token {
            address: token1,
            symbol: "DUMMY".to_string(),
            name: "DUMMY".to_string(),
            decimals: 18,
            chain_id: chain_id,
        },
        token1: Token {
            address: Address::from_str(WETH_TOKEN).unwrap(),
            symbol: "WETH".to_string(),
            name: "Wrapped Ether".to_string(),
            decimals: 18,
            chain_id: chain_id,
        },
        current_price: 0.0,
        dex: dex_for_price_reference.to_string(),
        chain_id: chain_id,
        price_levels: vec![],
        timestamp: Utc::now(),
    };
    let paired_token_addresses = [
        WETH_TOKEN,
        USDC_TOKEN,
        USDT_TOKEN,
        DAI_TOKEN,
        WBTC_TOKEN,
    ];
    
    let mut distributions = Vec::new();
    let mut usdc_pair_distribution = dummy_dist.clone();
    
    for &token_addr in &paired_token_addresses {
        // TODO: conduct this for all DEXes
        let distribution = match storage.get_liquidity_distribution(
            token1, Address::from_str(token_addr).unwrap(), dex_for_price_reference, chain_id
        )? {
            Some(dist) => dist,
            None => dummy_dist.clone(),
        };
        if token_addr == USDC_TOKEN {
            usdc_pair_distribution = distribution.clone();
        } 
        distributions.push(distribution);        
    }
    let mut ret = usdc_pair_distribution.price_levels.clone();
    
    if let Some((_, weth_price)) = token_prices.iter().find(|(addr, _)| *addr == Address::from_str(WETH_TOKEN).unwrap()) {
        info!("{} weth_price: {}", dex_for_price_reference ,weth_price);
    }
    if let Some((_, wbtc_price)) = token_prices.iter().find(|(addr, _)| *addr == Address::from_str(WBTC_TOKEN).unwrap()) {
        info!("wbtc_price: {}", wbtc_price);
    }
    if let Some((_, usdt_price)) = token_prices.iter().find(|(addr, _)| *addr == Address::from_str(USDT_TOKEN).unwrap()) {
        info!("usdt_price: {}", usdt_price);
    }
    if let Some((_, dai_price)) = token_prices.iter().find(|(addr, _)| *addr == Address::from_str(DAI_TOKEN).unwrap()) {
        info!("dai_price: {}", dai_price);
    }
    
    for dist in distributions {
        for (token_address, price) in token_prices.iter() {
            if dist.token1.address == *token_address {
                for mut price_level in dist.price_levels {
                    price_level.lower_price = price_level.lower_price * price;
                    price_level.upper_price = price_level.upper_price * price;
                    price_level.token1_liquidity = price_level.token1_liquidity * price;
                    price_level.token0_liquidity = price_level.token0_liquidity * dist.current_price;
                    if price_level.token0_liquidity < 0.0 || price_level.token1_liquidity <0.0 {
                        continue;
                    }
                    if price_level.side == Side::Sell{
                        price_level.token1_liquidity += price_level.token0_liquidity;
                        price_level.token0_liquidity = 0.0;
                    }
                    
                    info!("{} {} {:?} 0:{} 1:{}", dex_for_price_reference, dist.token1.symbol,price_level.side, price_level.token0_liquidity, price_level.token1_liquidity);
                    ret.push(price_level);
                }
                break;
            }
        }
    }
    // bucket price levels, sort price levels by lower price
    let bucket_size = 0.001;
    ret.sort_by(|a, b| a.lower_price.partial_cmp(&b.lower_price).unwrap());
    let mut bucketed_ret = Vec::new();
    for price_level in ret {
        let bucket_index = (price_level.lower_price / bucket_size).floor() as i32;
        bucketed_ret.push(PriceLiquidity {
            ..price_level
        });
    }
    ret = bucketed_ret;

    let mut aggregate_pool = usdc_pair_distribution.clone();
    aggregate_pool.price_levels = bucket_price_levels(ret, aggregate_pool.current_price, 0.001);
    for price in aggregate_pool.clone().price_levels{
        info!("bucket {:?} {}~{} 0:{} 1:{}",price.side,price.lower_price,price.upper_price,price.token0_liquidity,price.token1_liquidity);
    }
    let token1_name = Token1.name.clone();
    let token1_name = token1_name + "'s Aggregate Liquidity";
    aggregate_pool.token0 = Token {
        address: token1,
        symbol: Token1.symbol.clone(),
        name: token1_name,
        decimals: 18,
        chain_id: chain_id,
    };
    aggregate_pool.token1 = Token {
        address: Address::from_str(USDC_TOKEN).unwrap(),
        symbol: "USDC".to_string(),
        name: "USD Coin".to_string(),
        decimals: 6,
        chain_id: chain_id,
    };
    info!("current price for {} is {}", token1, aggregate_pool.current_price);
    Ok(aggregate_pool)
}

pub async fn save_token_async(storage: Arc<dyn Storage>, token: Token) -> Result<()> {
    storage.save_token(&token)
}

pub async fn get_token_async(
    storage: Arc<dyn Storage>,
    address: Address,
    chain_id: u64,
) -> Result<Option<Token>> {
    storage.get_token(address, chain_id)
}

/// Saves a pool to the storage asynchronously.
///
/// # Examples
///
/// ```
/// let storage = Arc::new(SqliteStorage::new(":memory:").unwrap());
/// let pool = Pool::default();
/// save_pool_async(storage, pool).await.unwrap();
/// ```
pub async fn save_pool_async(storage: Arc<dyn Storage>, pool: Pool) -> Result<()> {
    storage.save_pool(&pool)
}

/// Retrieves a pool by its address asynchronously.
///
/// # Examples
///
/// ```
/// let pool = get_pool_async(storage.clone(), pool_address).await?;
/// assert!(pool.is_some());
/// ```
pub async fn get_pool_async(storage: Arc<dyn Storage>, address: Address) -> Result<Option<Pool>> {
    storage.get_pool(address)
}

pub async fn save_liquidity_distribution_async(
    storage: Arc<dyn Storage>,
    distribution: LiquidityDistribution,
) -> Result<()> {
    storage.save_liquidity_distribution(&distribution)
}





