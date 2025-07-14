use crate::config::Config;
use crate::dexes::{get_dex_by_name, DexProtocol};
use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, Token};
use crate::providers::ProviderManager;
use crate::storage;
use crate::storage::Storage;
use crate::Address;
use alloy_primitives::U256;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, error, info, warn};

pub struct Indexer {
    config: Config,
    storage: Arc<dyn Storage>,
    provider_manager: Arc<ProviderManager>,
    dexes: HashMap<String, Box<dyn DexProtocol>>,
}

impl Indexer {
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

                if let Some(dex) = get_dex_by_name(&dex_config.name, provider, factory_address) {
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

    /// Runs the main indexing loop, periodically fetching and processing pools for all configured DEXes.
    ///
    /// The method continuously iterates over each enabled DEX, retrieves all pools, and processes each pool to update liquidity data. Errors encountered during pool retrieval or processing are logged, but do not interrupt the indexing cycle. The interval between cycles is determined by the configuration.
    ///
    /// This function runs indefinitely unless externally stopped.
    ///
    /// # Returns
    /// Returns `Ok(())` if the loop is started successfully. Errors are only returned if initial setup fails; runtime errors are logged and do not break the loop.
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

                // Only UniswapV2 skips on-chain fetch if DB already has pools.
                // UniswapV3 and others: always fetch and process pools to keep distributions up to date.
                if dex_name == "uniswap_v2" {
                    let pools_in_db = self.storage.get_pools_by_dex("uniswap_v2", dex.chain_id());
                    if let Ok(pools) = pools_in_db {
                        if !pools.is_empty() {
                            info!(
                                "DB에 이미 uniswap_v2 pool이 {}개 있으므로 on-chain 조회를 건너뜀",
                                pools.len()
                            );
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
                            continue;
                        }
                    }
                }

                // Get pools for this DEX
                if dex_name == "uniswap_v3" {
                    // Try to downcast to UniswapV3 and call get_all_pools_test
                    if let Some(uniswap_v3) = dex
                        .as_any()
                        .downcast_ref::<crate::dexes::uniswap_v3::UniswapV3>()
                    {
                        match uniswap_v3.get_all_pools_test().await {
                            Ok(pools) => {
                                info!(
                                    "[TEST] Found {} pools for {} (test mode)",
                                    pools.len(),
                                    dex_name
                                );
                                for pool in pools {
                                    match self.process_pool(&pool).await {
                                        Ok(_) => debug!(
                                            "Processed pool {} on {}",
                                            pool.address, pool.dex
                                        ),
                                        Err(e) => warn!(
                                            "Failed to process pool {} on {}: {}",
                                            pool.address, pool.dex, e
                                        ),
                                    }
                                }
                            }
                            Err(e) => warn!("[TEST] Failed to get pools for {}: {}", dex_name, e),
                        }
                    } else {
                        warn!("[TEST] Could not downcast dex to UniswapV3 for test pool fetch");
                    }
                } else {
                    match dex.get_all_pools().await {
                        Ok(pools) => {
                            info!("Found {} pools for {}", pools.len(), dex_name);
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
                        Err(e) => warn!("Failed to get pools for {}: {}", dex_name, e),
                    }
                }
            }
        }
    }

    /// Processes a liquidity pool by retrieving its liquidity distribution and saving it to storage.
    ///
    /// Returns an error if the DEX implementation is unknown or if fetching or saving the liquidity distribution fails.
    async fn process_pool(&self, pool: &Pool) -> Result<(), Error> {
        // Get DEX implementation
        let dex = self
            .dexes
            .get(&pool.dex)
            .ok_or_else(|| Error::UnknownDEX(pool.dex.clone()))?;

        // Get and store liquidity distribution
        if pool.dex == "uniswap_v3" {
            // V3: use V3LiquidityDistribution and save_v3_liquidity_distribution
            let v3_dist = dex
                .as_any()
                .downcast_ref::<crate::dexes::uniswap_v3::UniswapV3>()
                .ok_or_else(|| Error::UnknownDEX(pool.dex.clone()))?
                .get_v3_liquidity_distribution(pool.address)
                .await?;
            crate::storage::SqliteStorage::save_v3_liquidity_distribution(
                &*(self
                    .storage
                    .clone()
                    .downcast::<crate::storage::SqliteStorage>()
                    .map_err(|_| Error::DatabaseError("Failed to downcast storage".to_string()))?),
                &v3_dist,
            )?;
        } else {
            let distribution = dex.get_liquidity_distribution(pool.address).await?;
            storage::save_liquidity_distribution_async(self.storage.clone(), distribution).await?;
        }

        Ok(())
    }

    /// Indexes a specific liquidity pool for a given DEX and chain.
    ///
    /// Parses the pool address, retrieves pool details from the DEX implementation, and saves the pool data to storage.
    ///
    /// # Parameters
    /// - `dex`: The name of the DEX protocol.
    /// - `pool_address_str`: The string representation of the pool's address.
    /// - `chain_id`: The chain ID where the pool resides.
    ///
    /// # Returns
    /// The indexed `Pool` object on success.
    ///
    /// # Errors
    /// Returns an error if the address is invalid, the DEX is unknown, or if fetching or saving the pool fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let pool = indexer.index_pool("uniswap", "0x1234abcd...", 1).await?;
    /// assert_eq!(pool.dex, "uniswap");
    /// ```
    pub async fn index_pool(
        &self,
        dex: &str,
        pool_address_str: &str,
        chain_id: u64,
    ) -> Result<Pool, Error> {
        info!(
            "Indexing pool {} on {} (chain {})",
            pool_address_str, dex, chain_id
        );

        // Parse address
        let pool_address = Address::from_str(pool_address_str)
            .map_err(|_| Error::InvalidAddress(pool_address_str.to_string()))?;

        // Get DEX implementation
        let dex = self
            .dexes
            .get(dex)
            .ok_or_else(|| Error::UnknownDEX(dex.to_string()))?;

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
        let provider = self
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

    /// Retrieves the liquidity distribution for a specific pool on a given DEX.
    ///
    /// Parses the pool address, locates the DEX implementation, and fetches the liquidity distribution asynchronously. Returns an error if the DEX is unknown or the address is invalid.
    ///
    /// # Parameters
    /// - `dex`: The name of the DEX protocol.
    /// - `pool_address_str`: The string representation of the pool's address.
    ///
    /// # Returns
    /// The liquidity distribution for the specified pool.
    ///
    /// # Examples
    ///
    /// ```
    /// let distribution = indexer.get_liquidity_distribution("uniswap", "0x1234...abcd").await?;
    /// ```
    pub async fn get_liquidity_distribution(
        &self,
        dex: &str,
        pool_address_str: &str,
    ) -> Result<LiquidityDistribution, Error> {
        info!(
            "Getting liquidity distribution for {} on {}",
            pool_address_str, dex
        );

        // Parse address
        let pool_address = Address::from_str(pool_address_str)
            .map_err(|_| Error::InvalidAddress(pool_address_str.to_string()))?;

        // Get DEX implementation
        let dex = self
            .dexes
            .get(dex)
            .ok_or_else(|| Error::UnknownDEX(dex.to_string()))?;

        // Get liquidity distribution
        let distribution = dex.get_liquidity_distribution(pool_address).await?;

        Ok(distribution)
    }
}

/// Runs the indexer in either single pool mode or continuous indexing mode.
///
/// If both a DEX name and pool address are provided, indexes the specified pool and retrieves its liquidity distribution. Otherwise, starts the continuous indexing process for all configured DEXes and pools.
///
/// # Arguments
///
/// * `config` - The configuration for the indexer.
/// * `dex` - Optional DEX name to index a specific pool.
/// * `pair` - Optional pool address to index.
///
/// # Returns
///
/// Returns `Ok(())` if the indexing operation completes successfully, or an error if initialization or indexing fails.
///
/// # Examples
///
/// ```
/// let config = load_config();
/// // Run continuous indexing
/// run_indexer(config, None, None).await.unwrap();
///
/// // Index a specific pool
/// run_indexer(config, Some("uniswap".to_string()), Some("0x123...".to_string())).await.unwrap();
/// ```
pub async fn run_indexer(
    config: Config,
    dex: Option<String>,
    pair: Option<String>,
) -> Result<(), Error> {
    // Initialize the database connection
    let storage = Arc::new(crate::storage::SqliteStorage::new(&config.database.url)?);
    let indexer = Indexer::new(config, storage)?;

    match (dex, pair) {
        (Some(dex), Some(pool_address)) => {
            info!("Indexer running in single pool mode");

            // Validate DEX exists
            if !indexer.dexes.contains_key(&dex) {
                return Err(Error::UnknownDEX(dex));
            }

            // Find the chain ID for this DEX
            let chain_id = indexer
                .dexes
                .get(&dex)
                .map(|dex| dex.chain_id())
                .unwrap_or(1); // Default to Ethereum mainnet

            let pool = indexer.index_pool(&dex, &pool_address, chain_id).await?;
            info!("Indexed pool: {} on {}", pool.address, pool.dex);

            if dex == "uniswap_v3" {
                // V3 분기: V3LiquidityDistribution 저장
                let dex_impl = indexer.dexes.get(&dex).unwrap();
                let v3_dist = dex_impl
                    .as_any()
                    .downcast_ref::<crate::dexes::uniswap_v3::UniswapV3>()
                    .ok_or_else(|| Error::UnknownDEX(dex.clone()))?
                    .get_v3_liquidity_distribution(pool.address)
                    .await?;
                crate::storage::SqliteStorage::save_v3_liquidity_distribution(
                    &*(indexer
                        .storage
                        .clone()
                        .downcast::<crate::storage::SqliteStorage>()
                        .map_err(|_| {
                            Error::DatabaseError("Failed to downcast storage".to_string())
                        })?),
                    &v3_dist,
                )?;
            } else {
                // 기존 V2 방식
                match indexer
                    .get_liquidity_distribution(&dex, &pool_address)
                    .await
                {
                    Ok(distribution) => {
                        info!(
                            "Got liquidity distribution with {} price levels",
                            distribution.price_levels.len()
                        );
                        // Store distribution
                        if let Err(e) = storage::save_liquidity_distribution_async(
                            indexer.storage.clone(),
                            distribution,
                        )
                        .await
                        {
                            error!("Failed to store liquidity distribution: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to get liquidity distribution: {}", e);
                    }
                }
            }
        }
        _ => {
            // Run the continuous indexing process
            indexer.start().await?;
        }
    }

    Ok(())
}
