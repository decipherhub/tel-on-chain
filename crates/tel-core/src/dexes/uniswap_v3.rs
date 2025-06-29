use crate::dexes::DexProtocol;
use crate::error::Error;
use crate::models::{LiquidityDistribution, LiquidityTick, Pool, PriceLiquidity, Token};
use crate::providers::EthereumProvider;
use alloy_primitives::{Address, U256};
use ethers::types::{H256, U64, Log, BlockNumber, Filter};
use ethers::providers::Middleware;
use crate::storage::{
    get_pool_async, get_token_async, save_liquidity_distribution_async, save_pool_async,
    save_token_async, Storage,
};
use async_trait::async_trait;
use chrono::Utc;
use alloy_sol_types::sol;
use std::sync::Arc;
use crate::Result;

const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
const POOL_CREATED_SIG: &str = "PoolCreated(address,address,uint24,int24,address)";


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

        // (여기서부터) DB에 없을 때만 on-chain에서 메타데이터 조회 및 저장
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

        // 3) DB에 저장
        save_token_async(self.storage.clone(), token.clone()).await?;
        Ok(token)
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
        // TODO: Implement
        Err(Error::NotImplemented)
    }

    async fn get_all_pools(&self) -> Result<Vec<Pool>> {
        // This would require scanning events or getting pools from an indexer
        // For simplicity, returning empty vec
        // let inner = self.provider.provider();
        let factory_addr: Address = UNISWAP_V3_FACTORY.parse()
        .map_err(|e| Error::ProviderError(format!("Invalid factory address: {}", e)))?;
        let topic0 = H256::from_slice(&ethers::utils::keccak256(POOL_CREATED_SIG.as_bytes()));

        let filter = Filter::new()
            .address(factory_addr)
            .topic0(topic0)
            .from_block(0u64)
            .to_block(BlockNumber::Latest);

        // 3. Fetch logs
        let logs: Vec<Log> = provider.get_logs(&filter)
            .await
            .map_err(|e| Error::ProviderError(format!("get_logs: {}", e)))?;

        // 4. Take latest 10 logs
        let count = logs.len();
        let start = if count > 10 { count - 10 } else { 0 };
        let recent_logs = &logs[start..];

        // 5. Decode and process each log
        let mut pools = Vec::with_capacity(recent_logs.len());
        for log in recent_logs {
            // topics: [topic0, token0, token1, fee]
            let token0 = Address::from_slice(&log.topics[1].as_fixed_bytes()[12..]);
            let token1 = Address::from_slice(&log.topics[2].as_fixed_bytes()[12..]);
            // fee is uint24, in topics[3]
            let fee = log.topics[3].as_fixed_bytes()[29] as u32
                | ((log.topics[3].as_fixed_bytes()[30] as u32) << 8)
                | ((log.topics[3].as_fixed_bytes()[31] as u32) << 16);

            // data: [tickSpacing(int24)|poolAddress]
            let data = &log.data.0;
            // skip tickSpacing for now, only decode pool address
            let pool_addr = Address::from_slice(&data[32 + 12..32 + 32]);

            // 5-a. Fetch token metadata
            let tok0 = self.fetch_or_load_token(token0).await?;
            let tok1 = self.fetch_or_load_token(token1).await?;

            // 5-b. Build Pool struct
            let pool = Pool {
                address: pool_addr,
                dex: self.name().into(),
                chain_id: self.chain_id(),
                tokens: vec![tok0, tok1],
                creation_block: log.block_number
                    .unwrap_or(U64::zero())
                    .as_u64(),
                creation_timestamp: Utc::now(),
                last_updated_block: log.block_number
                    .unwrap_or(U64::zero())
                    .as_u64(),
                last_updated_timestamp: Utc::now(),
                fee,
            };

            // 5-c. Save to DB
            save_pool_async(self.storage.clone(), pool.clone()).await?;
            pools.push(pool);
        }
        Ok(pools)
    }

    async fn get_liquidity_distribution(
        &self,
        pool_address: Address,
    ) -> Result<LiquidityDistribution> {
        // TODO: Implement
        Err(Error::NotImplemented)
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
