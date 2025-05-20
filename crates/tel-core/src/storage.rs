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
    fn get_pools_by_dex(&self, dex_name: &str, chain_id: u64) -> Result<Vec<Pool>>;
    fn get_pools_by_token(&self, token_address: Address) -> Result<Vec<Pool>>;

    // Liquidity distribution operations
    fn save_liquidity_distribution(
        &self,
        distribution: &LiquidityDistribution,
    ) -> Result<()>;
    fn get_liquidity_distribution(
        &self,
        token0: Address,
        token1: Address,
        dex_name: &str,
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

    fn get_token(&self, address: Address, _chain_id: u64) -> Result<Option<Token>> {
        let _address_str = address.to_string();
        // TODO: Implement
        Ok(None)
    }

    fn save_pool(&self, pool: &Pool) -> Result<()> {
        let _address_str = pool.address.to_string();
        // TODO: Implement
        Ok(())
    }

    fn get_pool(&self, address: Address) -> Result<Option<Pool>> {
        let _address_str = address.to_string();
        // TODO: Implement
        Ok(None)
    }

    fn get_pools_by_dex(&self, _dex_name: &str, _chain_id: u64) -> Result<Vec<Pool>> {
        // TODO: Implement
        Ok(vec![])
    }

    fn get_pools_by_token(&self, token_address: Address) -> Result<Vec<Pool>> {
        let _address_str = token_address.to_string();
        // TODO: Implement
        Ok(vec![])
    }

    fn save_liquidity_distribution(
        &self,
        distribution: &LiquidityDistribution,
    ) -> Result<()> {
        let _token0_address_str = distribution.token0.address.to_string();
        let _token1_address_str = distribution.token1.address.to_string();
        // TODO: Implement
        Ok(())
    }

    fn get_liquidity_distribution(
        &self,
        token0: Address,
        token1: Address,
        _dex_name: &str,
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

pub async fn save_pool_async(storage: Arc<dyn Storage>, pool: Pool) -> Result<()> {
    storage.save_pool(&pool)
}

pub async fn get_pool_async(
    storage: Arc<dyn Storage>,
    address: Address,
) -> Result<Option<Pool>> {
    storage.get_pool(address)
}

pub async fn save_liquidity_distribution_async(
    storage: Arc<dyn Storage>,
    distribution: LiquidityDistribution,
) -> Result<()> {
    storage.save_liquidity_distribution(&distribution)
}
