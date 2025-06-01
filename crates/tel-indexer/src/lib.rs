use crate::storage::Storage;
use alloy_primitives::Address;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tel_core::config::Config;
use tel_core::dexes::{get_dex_by_name, DexProtocol};
use tel_core::error::Error;
use tel_core::models::{LiquidityDistribution, Pool, Token};
use tel_core::providers::ProviderManager;
use tel_core::storage;
use tel_core::storage::SqliteStorage;
use tokio::time;
use tracing::{debug, error, info, warn};

pub struct Indexer {
    config: Config,
    storage: Arc<dyn Storage>,
    provider_manager: Arc<ProviderManager>,
    dexes: HashMap<String, Box<dyn DexProtocol>>,
}

impl Indexer {
    /// Creates a new `Indexer` instance with configured providers and DEX implementations.
    ///
    /// Initializes the provider manager and loads enabled DEX protocols based on the provided configuration.
    /// Returns an error if provider initialization fails or if any DEX factory address is invalid. DEXes without implementations or providers are skipped with a warning.
    pub fn new(config: Config, storage: Arc<dyn Storage>) -> Result<Self, Error> {
        // Initialize provider manager from config
        let provider_manager = Arc::new(ProviderManager::new(
            &config.ethereum,
            config.polygon.as_ref(),
            config.arbitrum.as_ref(),
            config.optimism.as_ref(),
        )?);

        // Initialize DEX implementations
        let mut dexes = HashMap::new();
        for dex_config in &config.dexes {
            if !dex_config.enabled {
                continue;
            }

            if let Some(provider) = provider_manager.by_chain_id(dex_config.chain_id) {
                let factory_address = Address::from_str(&dex_config.factory_address)
                    .map_err(|_| Error::InvalidAddress(dex_config.factory_address.clone()))?;

                if let Some(dex) =
                    get_dex_by_name(&dex_config.name, provider, factory_address, storage.clone())
                {
                    dexes.insert(dex_config.name.clone(), dex);
                } else {
                    warn!("DEX implementation not found for: {}", dex_config.name);
                }
            } else {
                warn!(
                    "No provider available for chain ID {} (DEX: {})",
                    dex_config.chain_id, dex_config.name
                );
            }
        }

        Ok(Self {
            config,
            storage,
            provider_manager,
            dexes,
        })
    }

    /// Runs the indexer in continuous mode, periodically fetching and processing pools from all configured DEXes.
    ///
    /// This asynchronous method enters an infinite loop, retrieving pools from each DEX at the configured interval and processing their liquidity data. Errors encountered during pool retrieval or processing are logged, but do not interrupt the indexing cycle.
    ///
    /// # Returns
    /// Returns `Ok(())` if the loop is externally stopped; otherwise, runs indefinitely.
    pub async fn start(&self) -> Result<(), Error> {
        info!("Starting indexer...");
        let interval = Duration::from_secs(self.config.indexer.interval_secs);
        let mut interval_timer = time::interval(interval);

        loop {
            interval_timer.tick().await;
            info!("Indexer cycle running");

            // Process each configured DEX
            for (dex_name, dex) in &self.dexes {
                info!("Processing DEX: {}", dex_name);

                // Get pools for this DEX
                match dex.get_all_pools().await {
                    Ok(pools) => {
                        info!("Found {} pools for {}", pools.len(), dex_name);

                        // Process each pool
                        for pool in pools {
                            match self.process_pool(&pool).await {
                                Ok(_) => {
                                    debug!("Processed pool {} on {}", pool.address, pool.dex)
                                }
                                Err(e) => warn!(
                                    "Failed to process pool {} on {}: {}",
                                    pool.address, pool.dex, e
                                ),
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to get pools for {}: {}", dex_name, e);
                    }
                }
            }
        }
    }

    /// Processes a liquidity pool by retrieving and storing its liquidity distribution.
    ///
    /// Attempts to obtain the DEX implementation for the given pool, fetches the pool's liquidity distribution asynchronously, and saves the result to storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the DEX is unknown, if retrieving the liquidity distribution fails, or if saving to storage fails.
    async fn process_pool(&self, pool: &Pool) -> Result<(), Error> {
        // Get DEX implementation
        let dex = self
            .dexes
            .get(&pool.dex)
            .ok_or_else(|| Error::UnknownDEX(pool.dex.clone()))?;

        // Get and store liquidity distribution
        let distribution = dex.get_liquidity_distribution(pool.address).await?;
        storage::save_liquidity_distribution_async(self.storage.clone(), distribution).await?;

        Ok(())
    }

    pub async fn index_pool(
        &self,
        dex_name: &str,
        pool_address_str: &str,
        chain_id: u64,
    ) -> Result<Pool, Error> {
        info!(
            "Indexing pool {} on {} (chain {})",
            pool_address_str, dex_name, chain_id
        );

        // Parse address
        let pool_address = Address::from_str(pool_address_str)
            .map_err(|_| Error::InvalidAddress(pool_address_str.to_string()))?;

        // Get DEX implementation
        let dex = self
            .dexes
            .get(dex_name)
            .ok_or_else(|| Error::UnknownDEX(dex_name.to_string()))?;

        // Get pool details
        let pool = dex.get_pool(pool_address).await?;

        // Store in database
        storage::save_pool_async(self.storage.clone(), pool.clone()).await?;

        Ok(pool)
    }

    async fn get_or_create_token(&self, address: Address, chain_id: u64) -> Result<Token, Error> {
        // Try to get from database first
        if let Ok(Some(token)) =
            storage::get_token_async(self.storage.clone(), address, chain_id).await
        {
            return Ok(token);
        }

        // Get provider
        let _provider = self
            .provider_manager
            .by_chain_id(chain_id)
            .ok_or_else(|| Error::ProviderError(format!("No provider for chain {}", chain_id)))?;

        // We'll use the relevant DEX to get token information
        for dex in self.dexes.values() {
            if dex.chain_id() == chain_id {
                if let Ok(token) = dex.get_token(address).await {
                    // Store in database
                    storage::save_token_async(self.storage.clone(), token.clone()).await?;
                    return Ok(token);
                }
            }
        }

        // Fallback with unknown token info
        let token = Token {
            address,
            symbol: "UNKNOWN".to_string(),
            name: "Unknown Token".to_string(),
            decimals: 18,
            chain_id,
        };

        // Store in database
        storage::save_token_async(self.storage.clone(), token.clone()).await?;

        Ok(token)
    }

    pub async fn get_liquidity_distribution(
        &self,
        dex_name: &str,
        pool_address_str: &str,
    ) -> Result<LiquidityDistribution, Error> {
        info!(
            "Getting liquidity distribution for {} on {}",
            pool_address_str, dex_name
        );

        // Parse address
        let pool_address = Address::from_str(pool_address_str)
            .map_err(|_| Error::InvalidAddress(pool_address_str.to_string()))?;

        // Get DEX implementation
        let dex = self
            .dexes
            .get(dex_name)
            .ok_or_else(|| Error::UnknownDEX(dex_name.to_string()))?;

        // Get liquidity distribution
        let distribution = dex.get_liquidity_distribution(pool_address).await?;

        Ok(distribution)
    }
}

/// Runs the DEX indexer in either continuous or single-pool mode.
///
/// If both `dex` and `pair` are provided, indexes a specific pool for the given DEX and saves its liquidity distribution. Otherwise, starts the indexer in continuous mode to periodically index all configured DEXes and pools.
///
/// # Returns
/// Returns `Ok(())` on success, or an error if initialization or indexing fails.
///
/// # Examples
///
/// ```
/// let config = Config::default();
/// let result = run_indexer(config, Some("UniswapV2".to_string()), Some("0x1234...".to_string())).await;
/// assert!(result.is_ok());
/// ```
pub async fn run_indexer(
    config: Config,
    dex: Option<String>,
    pair: Option<String>,
) -> Result<(), Error> {
    // Initialize the database connection
    let storage = Arc::new(SqliteStorage::new(&config.database.url)?);
    let indexer = Indexer::new(config, storage)?;

    match (dex, pair) {
        (Some(dex_name), Some(pool_address)) => {
            info!("Indexer running in single pool mode");

            // Validate DEX exists
            if !indexer.dexes.contains_key(&dex_name) {
                return Err(Error::UnknownDEX(dex_name));
            }

            // Find the chain ID for this DEX
            let chain_id = indexer
                .dexes
                .get(&dex_name)
                .map(|dex| dex.chain_id())
                .unwrap_or(1); // Default to Ethereum mainnet

            let pool = indexer
                .index_pool(&dex_name, &pool_address, chain_id)
                .await?;
            info!("Indexed pool: {} on {}", pool.address, pool.dex);

            match indexer
                .get_liquidity_distribution(&dex_name, &pool_address)
                .await
            {
                Ok(distribution) => {
                    info!(
                        "Got liquidity distribution for pool {} on {}",
                        pool_address, dex_name
                    );
                    storage::save_liquidity_distribution_async(
                        indexer.storage.clone(),
                        distribution,
                    )
                    .await?;
                }
                Err(e) => {
                    error!(
                        "Failed to get liquidity distribution for pool {} on {}: {}",
                        pool_address, dex_name, e
                    );
                }
            }
        }
        _ => {
            info!("Indexer running in continuous mode");
            indexer.start().await?;
        }
    }

    Ok(())
}
