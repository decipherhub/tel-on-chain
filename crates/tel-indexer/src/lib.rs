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

// Only these pools are indexed in light mode!
pub const LIGHT_MODE_POOLS: [&str; 35] = [
    "0xb4e16d0168e52d35cacd2c6185b44281ec28c9dc",
    "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
    "0xCBCdF9626bC03E24f779434178A73a0B4bad62eD",
    "0x99ac8cA7087fA4A2A1FB6357269965A2014ABc35",
    "0xe8f7c89C5eFa061e340f2d2F206EC78FD8f7e124",
    "0x5777d92f208679DB4b9778590Fa3CAB3aC9e2168",
    "0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36",
    "0xC5c134A1f112efA96003f8559Dba6fAC0BA77692",
    "0x1d42064Fc4Beb5F8aAF85F4617AE8b3b5B8Bd801",
    "0x9Db9e0e53058C89e5B94e29621a205198648425B",
    "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8",
    "0x1d42064Fc4Beb5F8aAF85F4617AE8b3b5B8Bd801",
    "0xD0fC8bA7E267f2bc56044A7715A489d851dC6D78",
    "0x3470447f3cecffac709d3e783a307790b0208d60",
    "0xe0554a476a092703abdb3ef35c80e0d76d32939f",
    "0xa43fe16908251ee70ef74718545e4fe6c5ccec9f",
    "0x11950d141ecb863f01007add7d1a342041227b58",
    "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
    "0x3139Ffc91B99aa94DA8A2dc13f1fC36F9BDc98eE",
    "0x12EDE161c702D1494612d19f05992f43aa6A26FB",
    "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11",
    "0x07F068ca326a469Fc1d87d85d448990C8cBa7dF9",
    "0xAE461cA67B15dc8dc81CE7615e0320dA1A9aB8D5",
    "0xCe407CD7b95B39d3B4d53065E711e713dd5C5999",
    "0x33C2d48Bc95FB7D0199C5C693e7a9F527145a9Af",
    "0xB6909B960DbbE7392D405429eB2b3649752b4838",
    "0x30EB5E15476E6a80F4F3cd8479749b4881DAB1b8",
    "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc", //USDC/ETH
    "0xBb2b8038a1640196FbE3e38816F3e67Cba72D940", //WBTC/ETH
    "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852", //ETH/USDT
    "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11", //DAI/ETH
    "0xd3d2E2692501A5c9Ca623199D38826e513033a17", //UNI/ETH
    "0xd3d2E2692501A5c9Ca623199D38826e513033a17", //DAI/USDC
    "0xebfb684dd2b01e698ca6c14f10e4f289934a54d6", //UNI/USDC
    "0x5ac13261c181a9c3938bfe1b649e65d10f98566b", //UNI/USDT
];

pub const V2_POOLS: [&str; 9] = [
    "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc", //USDC/ETH
    "0xBb2b8038a1640196FbE3e38816F3e67Cba72D940", //WBTC/ETH
    "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852", //ETH/USDT
    "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11", //DAI/ETH
    "0xd3d2E2692501A5c9Ca623199D38826e513033a17", //UNI/ETH
    "0xd3d2E2692501A5c9Ca623199D38826e513033a17", //DAI/USDC
    "0xebfb684dd2b01e698ca6c14f10e4f289934a54d6", //UNI/USDC
    "0x5ac13261c181a9c3938bfe1b649e65d10f98566b", //UNI/USDT
    "0xA43fe16908251ee70EF74718545e4FE6C5cCEc9f", //PEPE/WETH
];

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
    pub async fn start(&self) {
        let light_mode: bool = true; // Only index first 10 pools for each dex. TODO: make it configurable

        if light_mode {
            info!(
                "Starting indexer in light mode... light_mode_pools: {:?}",
                LIGHT_MODE_POOLS
            );
        } else {
            info!("Starting indexer in full mode...");
        }
        let interval = Duration::from_secs(self.config.indexer.interval_secs);
        let mut interval_timer = time::interval(interval);

        loop {
            interval_timer.tick().await;
            info!("Indexer cycle running");

            // Process each configured DEX
            for (dex_name, dex) in &self.dexes {
                info!("Indexing pool states from DEX: {}", dex_name);

                // Get pools for this DEX
                match dex.get_all_pools_local().await {
                    Ok(pools) => {
                        info!("Found {} pools for {}", pools.len(), dex_name);
                        let pools: Vec<Pool> = if light_mode {
                            let light_mode_pools_addresses: Vec<Address> = LIGHT_MODE_POOLS
                                .iter()
                                .map(|addr| Address::from_str(addr).unwrap())
                                .collect();

                            pools
                                .into_iter()
                                .filter(|p| light_mode_pools_addresses.contains(&p.address))
                                .collect()
                        } else {
                            pools
                        };
                        for pool in pools {
                            match self.process_pool(&pool).await {
                                Ok(_) => debug!("Processed pool {} on {}", pool.address, pool.dex),
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

                info!("Finished indexing pool states from DEX: {}", dex_name);
            }
        }
    }

    pub async fn fetch(&self) -> Result<(), Error> {
        info!("Starting indexer fetch mode...");

        // Fetch all pools from each DEX
        for (dex_name, dex) in &self.dexes {
            // if dex_name == "uniswap_v2" {
            //     info!("Fetching pools for DEX: {}", dex_name);

            //     for pool_address in V2_POOLS{
            //         let pool_address = Address::from_str(pool_address)
            //                                 .map_err(|_| Error::InvalidAddress(pool_address.to_string()))?;
            //         match dex.get_pool(pool_address).await {
            //             Ok(pool) => {
            //                 match self.process_pool(&pool).await {
            //                     Ok(_) => debug!("Processed pool {} on {}", pool.address, pool.dex),
            //                     Err(e) => warn!(
            //                         "Failed to process pool {} on {}: {}",
            //                         pool.address, pool.dex, e
            //                     ),
            //                 }
            //             }
            //             Err(e) => {
            //                 warn!("Failed to fetch pools for {}: {}", dex_name, e);
            //             }
            //         }
            //     }
                
            // }
            match dex.get_all_pools().await {
                Ok(pools) => {
                    info!("Found {} pools for {}", pools.len(), dex_name);
                    for pool in pools {
                        match self.process_pool(&pool).await {
                            Ok(_) => debug!("Processed pool {} on {}", pool.address, pool.dex),
                            Err(e) => warn!(
                                "Failed to process pool {} on {}: {}",
                                pool.address, pool.dex, e
                            ),
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch pools for {}: {}", dex_name, e);
                }
            }
        }

        Ok(())
    }

    /// Processes a liquidity pool by retrieving and storing its liquidity distribution.
    ///
    /// Attempts to obtain the DEX implementation for the given pool, fetches the pool's liquidity distribution asynchronously, and saves the result to storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the DEX is unknown, if retrieving the liquidity distribution fails, or if saving to storage fails.
    async fn process_pool(&self, pool: &Pool) -> Result<(), Error> {
        let dex = self
            .dexes
            .get(&pool.dex)
            .ok_or_else(|| Error::UnknownDEX(pool.dex.clone()))?;

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
    test_mode: bool,
) -> Result<(), Error> {
    // Initialize the database connection
    let storage = Arc::new(SqliteStorage::new(&config.database.url)?);
    let indexer = Indexer::new(config, storage)?;

    match (dex, pair) {
        (Some(dex_name), Some(pool_address)) => {
            info!("Indexer running in single pool mode");
            if !indexer.dexes.contains_key(&dex_name) {
                return Err(Error::UnknownDEX(dex_name));
            }
            let chain_id = indexer
                .dexes
                .get(&dex_name)
                .map(|dex| dex.chain_id())
                .unwrap_or(1);
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
            indexer.start().await;
        }
    }

    Ok(())
}

pub async fn run_indexer_fetch(
    config: Config,
) -> Result<(), Error> {
    // Initialize the database connection
    let storage = Arc::new(SqliteStorage::new(&config.database.url)?);
    let indexer = Indexer::new(config, storage)?;

    info!("Indexer running in fetch mode");
    indexer.fetch().await?;
    

    Ok(())
}
