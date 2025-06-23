use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, Token};
use crate::Result;
use alloy_primitives::Address;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::task;

#[async_trait::async_trait]
pub trait Storage: Send + Sync {
    // Token operations
    fn save_token(&self, token: &Token) -> Result<()>;
    fn get_token(&self, address: Address, chain_id: u64) -> Result<Option<Token>>;

    // Pool operations
    fn save_pool(&self, pool: &Pool) -> Result<()>;
    fn get_pool(&self, address: Address) -> Result<Option<Pool>>;
    fn get_pools_by_dex(&self, dex: &str, chain_id: u64) -> Result<Vec<Pool>>;
    fn get_pools_by_token(&self, token_address: Address) -> Result<Vec<Pool>>;

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
        Ok(None)
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

        // ① 한 번만 연결 잠그고 트랜잭션 시작
        let mut conn = self.conn.lock().unwrap(); // ← mut 추가
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| Error::DatabaseError(format!("tx start: {e}")))?;

        // ② 토큰 2개 먼저 INSERT OR REPLACE
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

        // ③ 풀 INSERT
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
                pool.fee
            ],
        )
        .map_err(|e| Error::DatabaseError(format!("save_pool: {e}")))?;

        // ④ 커밋
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
        Ok(None)
    }

    /// Retrieves all pools for the specified DEX and chain ID.
    ///
    /// Currently unimplemented; always returns an empty vector.
    fn get_pools_by_dex(&self, _dex: &str, _chain_id: u64) -> Result<Vec<Pool>> {
        // TODO: Implement
        Ok(vec![])
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
    fn get_pools_by_token(&self, token_address: Address) -> Result<Vec<Pool>> {
        let _address_str = token_address.to_string();
        // TODO: Implement
        Ok(vec![])
    }

    /// Saves a liquidity distribution record to the storage.
    ///
    /// Currently unimplemented; calling this method has no effect and always returns success.
    fn save_liquidity_distribution(&self, distribution: &LiquidityDistribution) -> Result<()> {
        let _token0_address_str = distribution.token0.address.to_string();
        let _token1_address_str = distribution.token1.address.to_string();
        // TODO: Implement
        Ok(())
    }

    /// Retrieves the liquidity distribution for a given token pair, DEX, and chain ID.
    ///
    /// Returns `Ok(Some(LiquidityDistribution))` if a matching record exists, or `Ok(None)` if not found. Currently unimplemented and always returns `Ok(None)`.
    fn get_liquidity_distribution(
        &self,
        token0: Address,
        token1: Address,
        _dex: &str,
        _chain_id: u64,
    ) -> Result<Option<LiquidityDistribution>> {
        let _token0_str = token0.to_string();
        let _token1_str = token1.to_string();
        // TODO: Implement
        Ok(None)
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
