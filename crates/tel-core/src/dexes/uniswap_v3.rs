use crate::dexes::DexProtocol;
use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, PriceLiquidity, Token};
use crate::providers::EthereumProvider;
use crate::storage::{
    get_pool_async, get_token_async, save_liquidity_distribution_async, save_pool_async,
    save_token_async, Storage,
};
use crate::Result;
use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;
use async_trait::async_trait;
use chrono::Utc;
use std::str::FromStr;
use std::sync::Arc;

sol! {
    // ── Uniswap V3 Factory ───────────────────────────────────────────
    #[sol(rpc)]
    interface IUniswapV3Factory {
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool);
    }

    // ── Uniswap V3 Pool ──────────────────────────────────────────────
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );
        function liquidity() external view returns (uint128);
        function token0() external view returns (address);
        function token1() external view returns (address);
        function fee() external view returns (uint24);
    }

    // ── Uniswap V3 TickLens ──────────────────────────────────────────
    #[sol(rpc)]
    interface ITickLens {
        function getPopulatedTicksInWord(address pool, int16 wordPosition) external view returns (
            int24[] memory populatedTicks,
            uint256[] memory liquidityGross,
            uint256[] memory liquidityNet
        );
    }

    #[sol(rpc)]
    interface IERC20Metadata {
        function symbol()   external view returns (string);
        function name()     external view returns (string);
        function decimals() external view returns (uint8);
    }
}

const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
const POOL_CREATED_SIG: &str = "PoolCreated(address,address,uint24,int24,address)";
const HASH_POOL_CREATED: &str =
    "0x783cca1c0412dd0d695e784568c96da2e9c22ff989357a2e8b1d9b2b4e6b7118";
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
            storage,
        }
    }

    async fn fetch_or_load_token(&self, addr: Address) -> Result<Token> {
        let token_opt = get_token_async(self.storage.clone(), addr, self.chain_id()).await?;

        if let Some(tok) = token_opt {
            return Ok(tok);
        }

        // DB에 없을 때만 on-chain에서 메타데이터 조회 및 저장
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

        // DB에 저장
        save_token_async(self.storage.clone(), token.clone()).await?;
        Ok(token)
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

    fn tick_to_price(tick: i32, decimal0: u8, decimal1: u8) -> f64 {
        let base = 1.0001_f64;
        let tick_float = tick as f64;
        let price = base.powf(tick_float);

        // Apply decimal adjustment
        let decimal_adjustment = 10_f64.powi((decimal0 as i32).saturating_sub(decimal1 as i32));
        price * decimal_adjustment
    }

    // Get all active ticks for a pool using TickLens
    async fn get_active_ticks(&self, pool_address: Address) -> Result<Vec<(i32, u128, i128)>> {
        // TickLens contract address on Ethereum mainnet
        let tick_lens_address =
            Address::from_str("0xbfd8137f7d1516C3cB30cC1BcB3b3d7C3C3C3C3C3").unwrap();
        let tick_lens = ITickLens::new(tick_lens_address, self.provider.provider());

        let mut active_ticks = Vec::new();

        // Get current tick from slot0 to determine word range
        let pool = IUniswapV3Pool::new(pool_address, self.provider.provider());
        let slot0 = pool
            .slot0()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("slot0: {e}")))?;
        let current_tick = slot0.tick;

        let current_tick_i32 = current_tick.try_into().unwrap_or(0);
        let current_word = current_tick_i32 / 256;
        let word_range = 4; // Check ±4 words around current

        for word_pos in (current_word - word_range)..=(current_word + word_range) {
            let word_data = tick_lens
                .getPopulatedTicksInWord(pool_address, word_pos.try_into().unwrap_or(0))
                .call()
                .await;

            if let Ok(result) = word_data {
                for (i, &tick_idx) in result.populatedTicks.iter().enumerate() {
                    let liquidity_gross = result.liquidityGross[i].try_into().unwrap_or(0);
                    let liquidity_net = result.liquidityNet[i].try_into().unwrap_or(0);

                    // TickLens가 이미 활성화된 틱만 반환하므로 체크 불필요
                    active_ticks.push((
                        tick_idx.try_into().unwrap_or(0),
                        liquidity_gross,
                        liquidity_net,
                    ));
                }
            }
        }

        Ok(active_ticks)
    }

    // Calculate liquidity at a specific price level
    fn calculate_liquidity_at_price(
        &self,
        price: f64,
        token0_decimals: u8,
        token1_decimals: u8,
        liquidity_gross: u128,
    ) -> (f64, f64) {
        let sqrt_price = price.sqrt();
        let liquidity_float = liquidity_gross as f64;

        // Calculate token amounts based on price
        let token0_amount = liquidity_float / sqrt_price;
        let token1_amount = liquidity_float * sqrt_price;

        // Apply decimal adjustments
        let token0_liquidity = token0_amount / 10_f64.powi(token0_decimals as i32);
        let token1_liquidity = token1_amount / 10_f64.powi(token1_decimals as i32);

        (token0_liquidity, token1_liquidity)
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
        provider
            .get_logs(&filter)
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
        while from_block < latest_block && i < 10 {
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
            if pools_count >= 10 {
                break;
            }
            //info!("Processing log: topics={:?}, data={:?}", log.topics(), log.data());
            // topics: [topic0, token0, token1, fee]
            if log.topics().len() < 4 {
                continue;
            }
            let token0 = Address::from_slice(&log.topics()[1].as_slice()[12..]);
            let token1 = Address::from_slice(&log.topics()[2].as_slice()[12..]);
            let fee_bytes = log.topics()[3].as_slice();
            let fee = ((fee_bytes[29] as u32) << 16)
                | ((fee_bytes[30] as u32) << 8)
                | (fee_bytes[31] as u32);
            // data: [tickSpacing(int24)|poolAddress]
            let data_slice: &[u8] = log.data().data.as_ref();
            if data_slice.len() < 64 {
                continue;
            }
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
        let pool = self.get_pool(pool_address).await?;
        let token0 = &pool.tokens[0];
        let token1 = &pool.tokens[1];

        // Get current price from slot0
        let pool_contract = IUniswapV3Pool::new(pool_address, self.provider.provider());
        let slot0 = pool_contract
            .slot0()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("slot0: {e}")))?;

        let sqrt_price_x96 = U256::from(slot0.sqrtPriceX96);
        let current_price =
            Self::sqrt_price_x96_to_price(sqrt_price_x96, token0.decimals, token1.decimals);

        // Get all active ticks
        let active_ticks = self.get_active_ticks(pool_address).await?;

        let mut price_levels = Vec::new();

        // Add current price level
        let current_liquidity = pool_contract
            .liquidity()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("liquidity: {e}")))?;
        let current_liquidity_float = current_liquidity as f64;

        let (token0_liquidity, token1_liquidity) = self.calculate_liquidity_at_price(
            current_price,
            token0.decimals,
            token1.decimals,
            current_liquidity_float as u128,
        );

        price_levels.push(PriceLiquidity {
            price: current_price,
            token0_liquidity,
            token1_liquidity,
            timestamp: Utc::now(),
        });

        // Add price levels for each active tick
        for (tick_idx, liquidity_gross, _liquidity_net) in active_ticks {
            let tick_price = Self::tick_to_price(tick_idx, token0.decimals, token1.decimals);

            // Skip if price is too close to current price
            if (tick_price - current_price).abs() < 0.01 {
                continue;
            }

            let (token0_liquidity, token1_liquidity) = self.calculate_liquidity_at_price(
                tick_price,
                token0.decimals,
                token1.decimals,
                liquidity_gross,
            );

            price_levels.push(PriceLiquidity {
                price: tick_price,
                token0_liquidity,
                token1_liquidity,
                timestamp: Utc::now(),
            });
        }

        // Sort by price
        price_levels.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());

        let distribution = LiquidityDistribution {
            token0: token0.clone(),
            token1: token1.clone(),
            dex: self.name().to_string(),
            chain_id: self.chain_id(),
            price_levels,
            timestamp: Utc::now(),
        };

        save_liquidity_distribution_async(self.storage.clone(), distribution.clone()).await?;

        Ok(distribution)
    }

    async fn calculate_swap_impact(
        &self,
        _pool_address: Address,
        _token_in: Address,
        _amount_in: f64,
    ) -> Result<f64> {
        // TODO: Implement swap impact calculation
        Ok(0.0)
    }
}
