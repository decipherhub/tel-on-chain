use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, Token};
use crate::Result;
use alloy_primitives::Address;
use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{params, Connection};
use serde_json;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    // Token operations
    fn save_token(&self, token: &Token) -> Result<()>;
    fn get_token(&self, address: Address, chain_id: u64) -> Result<Option<Token>>;

    // Pool operations
    fn save_pool(&self, pool: &Pool) -> Result<()>;
    fn get_pool(&self, address: Address) -> Result<Option<Pool>>;
    fn get_pools_by_dex(&self, dex: &str, chain_id: u64) -> Result<Vec<Pool>>;
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
    ) -> Result<Option<LiquidityDistribution>>;
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
                "SELECT data FROM liquidity_distributions 
             WHERE token0_address = ?1 AND token1_address = ?2 AND dex = ?3 AND chain_id = ?4
             ORDER BY timestamp DESC LIMIT 1",
            )
            .map_err(|e| {
                Error::DatabaseError(format!("prepare get_liquidity_distribution: {e}"))
            })?;

        let distribution_opt = stmt.query_row(
            params![token0.to_string(), token1.to_string(), dex, chain_id],
            |row| {
                let data: String = row.get(0)?;
                let distribution: LiquidityDistribution = serde_json::from_str(&data)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                Ok(Some(distribution))
            },
        );

        match distribution_opt {
            Ok(distribution) => Ok(distribution),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::DatabaseError(format!(
                "get_liquidity_distribution error: {e}"
            ))),
        }
    }
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
