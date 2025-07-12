use crate::dexes::DexProtocol;
use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, PriceLiquidity, Token};
use crate::providers::EthereumProvider;
use alloy_primitives::{Address, B256, U256, U64};
use crate::storage::{    get_pool_async, get_token_async, save_liquidity_distribution_async, save_pool_async,    save_token_async, Storage,};
use async_trait::async_trait;
use chrono::Utc;
use alloy_sol_types::sol;
use std::sync::Arc;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use alloy_rpc_types::{Filter, Log};
use std::str::FromStr;
use alloy_provider::Provider; // Import the trait for get_filter_logs
use tracing::info;

const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
const POOL_CREATED_SIG: &str = "PoolCreated(address,address,uint24,int24,address)";
const HASH_POOL_CREATED: &str = "0x783cca1c0412dd0d695e784568c96da2e9c22ff989357a2e8b1d9b2b4e6b7118";
sol! {
    #[sol(rpc)]
    interface IERC20Metadata {
        function symbol()   external view returns (string);
        function name()     external view returns (string);
        function decimals() external view returns (uint8);
    }
}
pub struct UniswapV3 {
    provider: Arc<EthereumProvider>,
    factory_address: Address,
    storage: Arc<dyn Storage>,
}
impl UniswapV3 {

    pub fn new(
        provider: Arc<EthereumProvider>, 
        factory_address: Address,
        storage: Arc<dyn Storage>,
    ) -> Self {
        Self {
            provider,
            storage,
            factory_address,
        }
    }

    fn sqrt_price_x96_to_price(sqrt_price_x96: U256, decimal0: u8, decimal1: u8) -> f64 {
        let price = sqrt_price_x96.pow(U256::from(2));
        let decimal_adjustment = 10_u128.pow((decimal0 as u32).saturating_sub(decimal1 as u32));
        let base = 2_u128.pow(96 * 2);
        
        // Convert U256 to string and parse as f64
        let price_str = price.to_string();
        let price_f64 = price_str.parse::<f64>().unwrap_or(0.0);
        
        price_f64 / (base as f64) * (decimal_adjustment as f64)
    }

    // Helper to get information from a tick
    async fn get_tick_info(
        &self,
        _pool_address: Address,
        _tick_idx: i32,
    ) -> Result<PriceLiquidity> {
        // TODO: Implement
        Err(Error::NotImplemented)
    }

    async fn fetch_or_load_token(&self, addr: Address) -> Result<Token> {
        let token_opt = get_token_async(self.storage.clone(), addr, self.chain_id()).await?;

        if let Some(tok) = token_opt {
            return Ok(tok);
        }

        let erc20 = IERC20Metadata::new(addr, self.provider.provider());
        let symbol = erc20
            .symbol()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("{e}")))?;
        let name = erc20
            .name()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("{e}")))?;
        let decimals = erc20
            .decimals()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("{e}")))?;

        let token = Token {
            address: addr,
            symbol,
            name,
            decimals: decimals as u8,
            chain_id: self.chain_id(),
        };

        save_token_async(self.storage.clone(), token.clone()).await?;
        Ok(token)
    }

    /// Build a filter for PoolCreated events from the Uniswap V3 factory
    fn build_pool_created_filter(&self, from_block: u64, to_block: u64) -> Filter {
        Filter::new()
            .address(self.factory_address)
            .topic0(B256::from_str(HASH_POOL_CREATED).unwrap())
            .from_block(from_block)
            .to_block(to_block)
    }

    /// Fetch logs for a given filter
    async fn get_logs(&self, filter: Filter) -> Result<Vec<Log>> {
        let provider = self.provider.provider();
        provider.get_logs(&filter)
            .await
            .map_err(|e| Error::ProviderError(format!("get_logs: {}", e)))
    }
}

#[async_trait]
impl DexProtocol for UniswapV3 {
    fn name(&self) -> &str {
        "uniswap_v3"
    }

    fn chain_id(&self) -> u64 {
        self.provider.chain_id()
    }

    fn factory_address(&self) -> Address {
        self.factory_address
    }

    fn provider(&self) -> Arc<EthereumProvider> {
        self.provider.clone()
    }

    async fn get_pool(&self, pool_address: Address) -> Result<Pool> {
        // Mock implementation
        let dummy_token0 = Token {
            address: Address::ZERO,
            symbol: "MOCK0".to_string(),
            name: "Mock Token 0".to_string(),
            decimals: 18,
            chain_id: self.chain_id(),
        };
        let dummy_token1 = Token {
            address: Address::ZERO,
            symbol: "MOCK1".to_string(),
            name: "Mock Token 1".to_string(),
            decimals: 18,
            chain_id: self.chain_id(),
        };

        Ok(Pool {
            address: pool_address,
            dex: self.name().into(),
            chain_id: self.chain_id(),
            tokens: vec![dummy_token0, dummy_token1],
            creation_block: 0,
            creation_timestamp: Utc::now(),
            last_updated_block: 0,
            last_updated_timestamp: Utc::now(),
            fee: 3000,
        })
    }

    async fn get_all_pools(&self) -> Result<Vec<Pool>> {
        let provider = self.provider.provider();
        //let latest_block: u64 = provider.get_block_number().await.map_err(|e| Error::ProviderError(format!("get_block_number: {}", e)))?;
        let latest_block = 12500000; // For testing, replace with actual block number retrieval
        //let mut from_block = 12469621;
        let mut from_block = 12489621;

        let mut all_logs = Vec::new();
        let mut i = 0;
        while from_block < latest_block && i < 10{
            let to_block = (from_block + 9999).min(latest_block);
            //info!("Fetching logs from block {} to {}", from_block, to_block);
            let filter = self.build_pool_created_filter(from_block, to_block);
            let logs = self.get_logs(filter).await?;
            //info!("Found {} logs in this range", logs.len());
            all_logs.extend(logs);
            from_block = to_block + 1;
            i += 1;
        }

        //info!("Found a total of {} pools", all_logs.len());

        let mut pools = Vec::with_capacity(all_logs.len());
        let mut pools_count = 0;
        for log in &all_logs {
            if pools_count >= 10 { break; }
            //info!("Processing log: topics={:?}, data={:?}", log.topics(), log.data());
            // topics: [topic0, token0, token1, fee]
            if log.topics().len() < 4 { continue; }
            let token0 = Address::from_slice(&log.topics()[1].as_slice()[12..]);
            let token1 = Address::from_slice(&log.topics()[2].as_slice()[12..]);
            let fee_bytes = log.topics()[3].as_slice();
            let fee = ((fee_bytes[29] as u32) << 16)
                | ((fee_bytes[30] as u32) << 8)
                | (fee_bytes[31] as u32);
            // data: [tickSpacing(int24)|poolAddress]
            let data_slice: &[u8] = log.data().data.as_ref();
            if data_slice.len() < 64 { continue; }
            let pool_addr = Address::from_slice(&data_slice[44..64]);
            let tok0 = self.fetch_or_load_token(token0).await?;
            let tok1 = self.fetch_or_load_token(token1).await?;
            let block_number: u64 = log.block_number.unwrap_or(0);
            let pool = Pool {
                address: pool_addr,
                dex: self.name().into(),
                chain_id: self.chain_id(),
                tokens: vec![tok0, tok1],
                creation_block: block_number,
                creation_timestamp: Utc::now(),
                last_updated_block: block_number,
                last_updated_timestamp: Utc::now(),
                fee: fee as u64,
            };
            save_pool_async(self.storage.clone(), pool.clone()).await?;
            pools.push(pool);
            pools_count += 1;
        }
        Ok(pools)
    }

    async fn get_liquidity_distribution(
        &self,
        pool_address: Address,
    ) -> Result<LiquidityDistribution> {
        // Mock implementation
        let dummy_token0 = Token {
            address: Address::ZERO,
            symbol: "MOCK0".to_string(),
            name: "Mock Token 0".to_string(),
            decimals: 18,
            chain_id: self.chain_id(),
        };
        let dummy_token1 = Token {
            address: Address::ZERO,
            symbol: "MOCK1".to_string(),
            name: "Mock Token 1".to_string(),
            decimals: 18,
            chain_id: self.chain_id(),
        };

        let price_levels = vec![
            PriceLiquidity {
                price: 1.0,
                token0_liquidity: 100.0,
                token1_liquidity: 100.0,
                timestamp: Utc::now(),
            },
            PriceLiquidity {
                price: 1.05,
                token0_liquidity: 90.0,
                token1_liquidity: 110.0,
                timestamp: Utc::now(),
            },
            PriceLiquidity {
                price: 0.95,
                token0_liquidity: 110.0,
                token1_liquidity: 90.0,
                timestamp: Utc::now(),
            },
        ];

        Ok(LiquidityDistribution {
            token0: dummy_token0,
            token1: dummy_token1,
            dex: self.name().into(),
            chain_id: self.chain_id(),
            price_levels,
            timestamp: Utc::now(),
        })
    }

    async fn calculate_swap_impact(
        &self,
        _pool_address: Address,
        _token_in: Address,
        _amount_in: f64,
    ) -> Result<f64> {
        // TODO: Implement
        Err(Error::NotImplemented)
    }
}
