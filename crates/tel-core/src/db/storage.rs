use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, Token};
use crate::Address;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::task;

// Simplified storage trait without async for compatibility
pub trait Storage: Send + Sync {
    // Token operations
    fn save_token(&self, token: &Token) -> Result<(), Error>;
    fn get_token(&self, address: Address, chain_id: u64) -> Result<Option<Token>, Error>;

    // Pool operations
    fn save_pool(&self, pool: &Pool) -> Result<(), Error>;
    fn get_pool(&self, address: Address) -> Result<Option<Pool>, Error>;
    fn get_pools_by_dex(&self, dex_name: &str, chain_id: u64) -> Result<Vec<Pool>, Error>;
    fn get_pools_by_token(&self, token_address: Address) -> Result<Vec<Pool>, Error>;

    // Liquidity distribution operations
    fn save_liquidity_distribution(
        &self,
        distribution: &LiquidityDistribution,
    ) -> Result<(), Error>;
    fn get_latest_liquidity_distribution(
        &self,
        token0: Address,
        token1: Address,
        dex_name: &str,
        chain_id: u64,
    ) -> Result<Option<LiquidityDistribution>, Error>;
}

pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    pub fn new(database_path: &str) -> Result<Self, Error> {
        // Check if database file exists, create parent directories if needed
        let path = Path::new(database_path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    Error::DatabaseError(format!("Failed to create directory: {}", e))
                })?;
            }
        }

        // Open or create database
        let conn = Connection::open(database_path)
            .map_err(|e| Error::DatabaseError(format!("Failed to open database: {}", e)))?;

        // Initialize the database schema
        Self::init_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_schema(conn: &Connection) -> Result<(), Error> {
        // Initialize database schema
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                address TEXT PRIMARY KEY,
                chain_id INTEGER,
                name TEXT,
                symbol TEXT,
                decimals INTEGER
            )",
            [],
        )
        .map_err(|e| Error::DatabaseError(format!("Failed to create tokens table: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS pools (
                address TEXT PRIMARY KEY,
                chain_id INTEGER,
                dex_name TEXT,
                token0_address TEXT,
                token1_address TEXT,
                fee INTEGER,
                FOREIGN KEY (token0_address) REFERENCES tokens (address),
                FOREIGN KEY (token1_address) REFERENCES tokens (address)
            )",
            [],
        )
        .map_err(|e| Error::DatabaseError(format!("Failed to create pools table: {}", e)))?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS liquidity_distributions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                pool_address TEXT,
                token0_address TEXT,
                token1_address TEXT,
                dex_name TEXT,
                chain_id INTEGER,
                timestamp INTEGER,
                distribution_json TEXT,
                FOREIGN KEY (pool_address) REFERENCES pools (address)
            )",
            [],
        )
        .map_err(|e| {
            Error::DatabaseError(format!(
                "Failed to create liquidity_distributions table: {}",
                e
            ))
        })?;

        Ok(())
    }
}

impl Storage for SqliteStorage {
    fn save_token(&self, token: &Token) -> Result<(), Error> {
        // Simple implementation to insert or replace token
        let conn = self.conn.lock().unwrap();
        // Convert Address to String for storage
        let address_str = token.address.to_string();
        conn.execute(
            "INSERT OR REPLACE INTO tokens (address, chain_id, name, symbol, decimals) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                address_str,
                token.chain_id,
                token.name,
                token.symbol,
                token.decimals
            ],
        )
        .map_err(|e| Error::DatabaseError(format!("Failed to save token: {}", e)))?;
        Ok(())
    }

    fn get_token(&self, address: Address, chain_id: u64) -> Result<Option<Token>, Error> {
        // Convert Address to String for querying
        let address_str = address.to_string();
        // In a real implementation, we would query the database for the token
        Ok(None)
    }

    fn save_pool(&self, pool: &Pool) -> Result<(), Error> {
        // Convert Address to String for storage
        let address_str = pool.address.to_string();
        // In a real implementation, we would insert the pool into the database
        Ok(())
    }

    fn get_pool(&self, address: Address) -> Result<Option<Pool>, Error> {
        // Convert Address to String for querying
        let address_str = address.to_string();
        // In a real implementation, we would query the database for the pool
        Ok(None)
    }

    fn get_pools_by_dex(&self, dex_name: &str, chain_id: u64) -> Result<Vec<Pool>, Error> {
        // In a real implementation, we would query the database for pools
        Ok(Vec::new())
    }

    fn get_pools_by_token(&self, token_address: Address) -> Result<Vec<Pool>, Error> {
        // Convert Address to String for querying
        let address_str = token_address.to_string();
        // In a real implementation, we would query the database for pools
        Ok(Vec::new())
    }

    fn save_liquidity_distribution(
        &self,
        distribution: &LiquidityDistribution,
    ) -> Result<(), Error> {
        // Convert Address to String for storage
        let token0_address_str = distribution.token0.address.to_string();
        let token1_address_str = distribution.token1.address.to_string();
        // In a real implementation, we would insert the distribution into the database
        Ok(())
    }

    fn get_latest_liquidity_distribution(
        &self,
        token0: Address,
        token1: Address,
        dex_name: &str,
        chain_id: u64,
    ) -> Result<Option<LiquidityDistribution>, Error> {
        // Convert Address to String for querying
        let token0_str = token0.to_string();
        let token1_str = token1.to_string();
        // In a real implementation, we would query the database for the latest distribution
        Ok(None)
    }
}

// Helper methods for async calling into sync API
pub async fn save_token_async(storage: Arc<dyn Storage>, token: Token) -> Result<(), Error> {
    let storage_clone = Arc::clone(&storage);
    task::spawn_blocking(move || storage_clone.save_token(&token))
        .await
        .unwrap()
}

pub async fn get_token_async(
    storage: Arc<dyn Storage>,
    address: Address,
    chain_id: u64,
) -> Result<Option<Token>, Error> {
    let storage_clone = Arc::clone(&storage);
    task::spawn_blocking(move || storage_clone.get_token(address, chain_id))
        .await
        .unwrap()
}

pub async fn save_pool_async(storage: Arc<dyn Storage>, pool: Pool) -> Result<(), Error> {
    let storage_clone = Arc::clone(&storage);
    task::spawn_blocking(move || storage_clone.save_pool(&pool))
        .await
        .unwrap()
}

pub async fn get_pool_async(
    storage: Arc<dyn Storage>,
    address: Address,
) -> Result<Option<Pool>, Error> {
    let storage_clone = Arc::clone(&storage);
    task::spawn_blocking(move || storage_clone.get_pool(address))
        .await
        .unwrap()
}

pub async fn save_liquidity_distribution_async(
    storage: Arc<dyn Storage>,
    distribution: LiquidityDistribution,
) -> Result<(), Error> {
    let storage_clone = Arc::clone(&storage);
    task::spawn_blocking(move || storage_clone.save_liquidity_distribution(&distribution))
        .await
        .unwrap()
}
