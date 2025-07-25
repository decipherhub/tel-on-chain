use crate::dexes::DexProtocol;
use crate::error::Error;
use crate::models::{
    LiquidityDistribution, Pool, PriceLiquidity, Side, Token, V3LiquidityDistribution,
    V3PriceLevel, V3PriceLiquidity,
};
use crate::providers::EthereumProvider;
use crate::storage::{get_pool_async, get_token_async, save_pool_async, save_token_async, Storage};
use crate::Result;
use alloy_primitives::aliases::I24;
use alloy_primitives::U160;
use alloy_primitives::{Address, B256, U256};
use alloy_provider::Provider;
use alloy_rpc_types::{Filter, Log};
use alloy_sol_types::sol;
use async_trait::async_trait;
use chrono::Utc;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use uniswap_sdk_core::prelude::{CurrencyAmount, FractionBase, Rounding};
use uniswap_sdk_core::{prelude::*, token};
use uniswap_v3_sdk::entities::{Tick, TickListDataProvider};
use uniswap_v3_sdk::prelude::*;
use uniswap_v3_sdk::utils::tick_math::{MAX_TICK, MIN_TICK};

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
        function tickSpacing() external view returns (int24);
    }

    // ── TickInfo struct for TickLens ────────────────────────────────
    #[derive(Debug)]
    struct TickInfo {
        int24 tick;
        uint128 liquidityGross;
        int128 liquidityNet;
    }

    // ── Uniswap V3 TickLens ──────────────────────────────────────────
    #[sol(rpc)]
    interface ITickLens {
        #[derive(Debug)]
        function getPopulatedTicksInWord(address pool, int16 wordPosition) external view returns (
            TickInfo[] memory populatedTicks
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
pub struct UniswapV3 {
    provider: Arc<EthereumProvider>,
    factory_address: Address,
    storage: Arc<dyn Storage>,
}

impl UniswapV3 {
    /// Create a new UniswapV3 instance
    pub fn new(
        provider: Arc<EthereumProvider>,
        factory_address: Address,
        storage: Arc<dyn Storage>,
    ) -> Self {
        Self {
            provider,
            factory_address,
            storage,
        }
    }

    /// Fetch token from DB or on-chain if not present
    async fn fetch_or_load_token(&self, addr: Address) -> Result<Token> {
        if let Some(tok) = get_token_async(self.storage.clone(), addr, self.chain_id()).await? {
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

    /// Convert tick index to price, adjusting for token decimals
    fn tick_to_price(tick: i32, decimal0: u8, decimal1: u8) -> f64 {
        let price = 1.0001_f64.powf(tick as f64);
        let decimal_adjustment = 10f64.powi((decimal0 as i32).saturating_sub(decimal1 as i32));
        price * decimal_adjustment
    }

    /// Fetch all active ticks for a pool using TickLens
    async fn get_active_ticks(
        &self,
        pool_address: Address,
    ) -> Result<(i32, Vec<(i32, u128, i128)>)> {
        let tick_lens_address =
            Address::from_str("0xbfd8137f7d1516D3ea5cA83523914859ec47F573").unwrap();
        let tick_lens = ITickLens::new(tick_lens_address, self.provider.provider());
        let pool = IUniswapV3Pool::new(pool_address, self.provider.provider());
        let slot0 = pool
            .slot0()
            .call()
            .await
            .map_err(|e| crate::Error::ProviderError(format!("slot0: {e}")))?;
        let tick_spacing: i32 = pool
            .tickSpacing()
            .call()
            .await
            .map_err(|e| crate::Error::ProviderError(format!("tickSpacing: {e}")))?
            .try_into()
            .unwrap_or(1);
        let current_tick: i32 = slot0.tick.try_into().unwrap_or(0);
        let current_word = (current_tick / tick_spacing) >> 8;
        let mut active_ticks = Vec::new();
        if current_word >= i16::MIN as i32 && current_word <= i16::MAX as i32 {
            let word_i16 = current_word as i16;
            let call_result = tick_lens
                .getPopulatedTicksInWord(pool_address, word_i16)
                .call()
                .await;
            if let Ok(result) = call_result {
                for tick_info in result {
                    let tick_idx: i32 = tick_info.tick.try_into().unwrap_or(0);
                    let liquidity_gross: u128 = tick_info.liquidityGross.try_into().unwrap_or(0);
                    let liquidity_net: i128 = tick_info.liquidityNet.try_into().unwrap_or(0);
                    active_ticks.push((tick_idx, liquidity_gross, liquidity_net));
                }
            }
        }
        active_ticks.sort_by_key(|(tick, _, _)| *tick);
        Ok((current_tick, active_ticks))
    }

    /// Build a filter for PoolCreated events
    fn build_pool_created_filter(&self, from_block: u64, to_block: u64) -> Filter {
        Filter::new()
            .address(self.factory_address)
            .event_signature(B256::from_str(HASH_POOL_CREATED).unwrap())
            .from_block(from_block)
            .to_block(to_block)
    }

    /// Fetch logs for a given filter
    async fn get_logs(&self, filter: Filter) -> Result<Vec<Log>> {
        self.provider
            .provider()
            .get_logs(&filter)
            .await
            .map_err(|e| Error::ProviderError(format!("get_logs: {}", e)))
    }

    /// Return an empty LiquidityDistribution
    fn empty_dist(
        token0: &Token,
        token1: &Token,
        dex: &str,
        chain_id: u64,
    ) -> LiquidityDistribution {
        LiquidityDistribution {
            token0: token0.clone(),
            token1: token1.clone(),
            current_price: 0.0,
            dex: dex.to_string(),
            chain_id,
            price_levels: vec![],
            timestamp: Utc::now(),
        }
    }

    /// Return an empty V3LiquidityDistribution
    fn empty_v3_dist(
        token0: &Token,
        token1: &Token,
        dex: &str,
        chain_id: u64,
    ) -> V3LiquidityDistribution {
        V3LiquidityDistribution {
            token0: token0.clone(),
            token1: token1.clone(),
            dex: dex.to_string(),
            chain_id,
            current_tick: 0,
            price_levels: vec![],
            timestamp: Utc::now(),
        }
    }

    // Add a new async function for active range calculation
    async fn calculate_active_range_tokens_locked(
        &self,
        tick_idx: i32,
        liq_gross: u128,
        liq_net: i128,
        tick_spacing: i32,
        fee: u32,
        slot0_sqrt_price_x96: U256,
        uni_token0: &uniswap_sdk_core::prelude::Token,
        uni_token1: &uniswap_sdk_core::prelude::Token,
    ) -> std::result::Result<V3PriceLevel, crate::Error> {
        use uniswap_v3_sdk::prelude::*;
        use uniswap_v3_sdk::utils::price_tick_conversions::tick_to_price;
        let mock_ticks = vec![
            Tick {
                index: I24::try_from(tick_idx).unwrap(),
                liquidity_gross: liq_gross,
                liquidity_net: -liq_net,
            },
            Tick {
                index: I24::try_from(tick_idx + tick_spacing).unwrap(),
                liquidity_gross: liq_gross,
                liquidity_net: liq_net,
            },
        ];
        let pool_sim = Pool::new_with_tick_data_provider(
            uni_token0.clone(),
            uni_token1.clone(),
            FeeAmount::try_from(fee).unwrap_or(FeeAmount::MEDIUM),
            U160::from(slot0_sqrt_price_x96.to::<u128>()),
            liq_gross,
            TickListDataProvider::new(mock_ticks.clone(), I24::try_from(tick_spacing).unwrap()),
        )
        .map_err(|e| crate::Error::ProviderError(format!("Pool: {e}")))?;

        // bottomOfRangePrice
        let bottom_sqrt_x96 = {
            let sqrt: U256 = TickMath::get_sqrt_ratio_at_tick(I24::try_from(tick_idx).unwrap())
                .map_err(|e| crate::Error::ProviderError(format!("TickMath: {e}")))?;
            sqrt.to::<u128>()
        };
        let max_amount_token0 = CurrencyAmount::from_raw_amount(uni_token0.clone(), u128::MAX)
            .map_err(|e| crate::Error::ProviderError(format!("CurrencyAmount: {e}")))?;
        let token1_amount = pool_sim
            .get_output_amount(&max_amount_token0, Some(U160::from(bottom_sqrt_x96)))
            .await
            .map_err(|e| crate::Error::ProviderError(format!("get_output_amount: {e}")))?;
        let price = tick_to_price(
            uni_token0.clone(),
            uni_token1.clone(),
            I24::try_from(tick_idx).unwrap(),
        )
        .map_err(|e| crate::Error::ProviderError(format!("tick_to_price: {e}")))?;
        let amount0_locked = match price.invert().quote(&token1_amount) {
            Ok(q) => q.to_exact().parse::<f64>().unwrap_or(0.0),
            Err(e) => return Err(crate::Error::ProviderError(format!("quote error: {e}"))),
        };

        // topOfRangePrice
        let top_sqrt_x96 = {
            let sqrt: U256 =
                TickMath::get_sqrt_ratio_at_tick(I24::try_from(tick_idx + tick_spacing).unwrap())
                    .map_err(|e| crate::Error::ProviderError(format!("TickMath: {e}")))?;
            sqrt.to::<u128>()
        };
        let max_amount_token1 = CurrencyAmount::from_raw_amount(uni_token1.clone(), u128::MAX)
            .map_err(|e| crate::Error::ProviderError(format!("CurrencyAmount: {e}")))?;
        let token0_amount = pool_sim
            .get_output_amount(&max_amount_token1, Some(U160::from(top_sqrt_x96)))
            .await
            .map_err(|e| crate::Error::ProviderError(format!("get_output_amount: {e}")))?;
        let amount1_locked = match price.quote(&token0_amount) {
            Ok(q) => q.to_exact().parse::<f64>().unwrap_or(0.0),
            Err(e) => return Err(crate::Error::ProviderError(format!("quote error: {e}"))),
        };

        Ok(V3PriceLevel {
            tick_idx,
            price: price
                .to_significant(12, Some(Rounding::RoundDown))
                .unwrap_or_else(|_| "0".to_string())
                .parse::<f64>()
                .unwrap_or(0.0),
            tick_price: 1.0001_f64.powi(tick_idx),
            token0_liquidity: amount0_locked,
            token1_liquidity: amount1_locked,
            timestamp: Utc::now(),
        })
    }
}

// --- Uniswap v3 math utilities (see uniswap-v3-sdk-rs) ---
impl UniswapV3 {}

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

    fn storage(&self) -> Arc<dyn Storage> {
        self.storage.clone()
    }

    async fn get_pool(&self, pool_address: Address) -> crate::Result<Pool> {
        let pool_result = get_pool_async(self.storage.clone(), pool_address).await;
        match pool_result {
            Ok(Some(pool)) => Ok(pool),
            Ok(None) => Err(Error::DexError(format!("Pool not found: {}", pool_address))),
            Err(e) => Err(e),
        }
    }

    async fn get_all_pools(&self) -> Result<Vec<Pool>> {
        self.get_all_pools_test().await
        // let provider = self.provider.provider();
        // let latest_block: u64 = provider
        //     .get_block_number()
        //     .await
        //     .map_err(|e| Error::ProviderError(format!("get_block_number: {}", e)))?;
        // //let latest_block = 16669621; // For testing, replace with actual block number retrieval
        // let mut from_block = 12469621;
        // let mut all_logs = Vec::new();
        // let mut i = 0;
        // while from_block < latest_block {
        //     let to_block = (from_block + 9999).min(latest_block);
        //     info!("Fetching logs from block {} to {}", from_block, to_block);

        //     let filter = self.build_pool_created_filter(from_block, to_block);
        //     let logs = self.get_logs(filter).await?;
        //     all_logs.extend(logs);
        //     from_block = to_block + 1;
        //     i += 1;
        // }

        // info!("Found a total of {} pools", all_logs.len());

        let mut pools = Vec::with_capacity(all_logs.len());
        let mut pools_count = 0;
        for log in &all_logs {
            //if pools_count >= 10 { break; }
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
            if data_slice.len() < 64 {
                continue;
            }
            let pool_addr = Address::from_slice(&data_slice[44..64]);
            let tok0 = match self.fetch_or_load_token(token0).await {
                Ok(token) => token,
                Err(e) => {
                    tracing::error!("Failed to fetch token {}: {}", token0, e);
                    continue;
                }
            };
            let tok1 = match self.fetch_or_load_token(token1).await {
                Ok(token) => token,
                Err(e) => {
                    tracing::error!("Failed to fetch token {}: {}", token1, e);
                    continue;
                }
            };
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
            let pool_address = pool.address;
            if let Err(e) = save_pool_async(self.storage.clone(), pool.clone()).await {
                tracing::error!("Failed to save pool {}: {}", pool_address, e);
                continue;
            };

            pools.push(pool);
            info!("Pool {}: token0={}, token1={}, fee={}", pool_address, token0, token1, fee);
            pools_count += 1;
        }
        Ok(pools)
    }

    async fn get_liquidity_distribution(
        &self,
        pool_address: Address,
    ) -> Result<LiquidityDistribution> {
        let v3_dist = self.get_v3_liquidity_distribution(pool_address).await?;
        let current_price = Self::tick_to_price(
            v3_dist.current_tick,
            v3_dist.token0.decimals,
            v3_dist.token1.decimals,
        );
        let price_levels = v3_dist
            .price_levels
            .iter()
            .map(|lvl| PriceLiquidity {
                side: if lvl.price < current_price {
                    Side::Buy
                } else {
                    Side::Sell
                },
                lower_price: lvl.price,
                upper_price: lvl.price,
                token0_liquidity: lvl.token0_liquidity,
                token1_liquidity: lvl.token1_liquidity,
                timestamp: lvl.timestamp,
            })
            .collect();

        Ok(LiquidityDistribution {
            token0: v3_dist.token0.clone(),
            token1: v3_dist.token1.clone(),
            current_price,
            dex: v3_dist.dex.clone(),
            chain_id: v3_dist.chain_id,
            price_levels,
            timestamp: v3_dist.timestamp,
        })
    }

    async fn calculate_swap_impact(
        &self,
        _pool_address: Address,
        _token_in: Address,
        _amount_in: f64,
    ) -> Result<f64> {
        Ok(0.0)
    }

    // get_v3_liquidity_distribution: behaves identically to the JS SDK implementation
    async fn get_v3_liquidity_distribution(
        &self,
        pool_address: Address,
    ) -> std::result::Result<V3LiquidityDistribution, Error> {
        use uniswap_v3_sdk::prelude::*;
        use uniswap_v3_sdk::utils::price_tick_conversions::tick_to_price;
        // --- Load metadata as in JS ---
        let pool = match self.get_pool(pool_address).await {
            Ok(p) => p,
            Err(_) => {
                let dummy = Token {
                    address: pool_address,
                    symbol: String::new(),
                    name: String::new(),
                    decimals: 0,
                    chain_id: self.chain_id(),
                };
                return Ok(Self::empty_v3_dist(
                    &dummy,
                    &dummy,
                    &self.name().to_lowercase(),
                    self.chain_id(),
                ));
            }
        };
        let token0 = &pool.tokens[0];
        let token1 = &pool.tokens[1];

        // ---- on‑chain 현재 sqrtPriceX96, tick, spacing ---------------------------
        let pool_contract = IUniswapV3Pool::new(pool_address, self.provider.provider());
        let slot0 = pool_contract
            .slot0()
            .call()
            .await
            .map_err(|e| crate::Error::ProviderError(format!("slot0: {e}")))?;
        let sqrt_price_x96: u128 = slot0.sqrtPriceX96.to();
        let current_tick: i32 = slot0.tick.try_into().unwrap_or(0);
        let tick_spacing: i32 = pool_contract
            .tickSpacing()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("tickSpacing: {e}")))?
            .try_into()
            .unwrap_or(1);
        let fee = pool.fee as u32; // Uniswap SDK expects u32
                                   // --- populated ticks ---
        let (_ignored, populated) = self.get_active_ticks(pool_address).await?;
        if populated.is_empty() {
            return Ok(Self::empty_v3_dist(
                token0,
                token1,
                &self.name().to_lowercase(),
                self.chain_id(),
            ));
        }
        // ===== DEBUG: Populated tick data =====
        println!("[DEBUG] Populated ticks: {:?}", populated);
        // ===== END DEBUG =====
        // Rust Token 변환 (SDK용)
        let uni_token0 = uniswap_sdk_core::prelude::Token::new(
            self.chain_id(),
            token0.address,
            token0.decimals,
            Some(token0.symbol.clone()),
            Some(token0.name.clone()),
            0,
            0,
        );
        let uni_token1 = uniswap_sdk_core::prelude::Token::new(
            self.chain_id(),
            token1.address,
            token1.decimals,
            Some(token1.symbol.clone()),
            Some(token1.name.clone()),
            0,
            0,
        );
        // uni_token0, uni_token1은 오직 Pool, CurrencyAmount 등 SDK 타입에만 사용하세요.
        let mut v3_levels = Vec::with_capacity(populated.len());
        for pair in populated.windows(2) {
            let (tick_idx_a, liq_gross_a, liq_net_a) = pair[0];
            let (tick_idx_b, liq_gross_b, liq_net_b) = pair[1];
            let net_diff = liq_net_b - liq_net_a;
            // ===== DEBUG: Per-tick-pair values =====
            println!(
                "[DEBUG] tick_idx_a: {}, liq_gross_a: {}, liq_net_a: {}, tick_idx_b: {}, liq_gross_b: {}, liq_net_b: {}, net_diff: {}",
                tick_idx_a, liq_gross_a, liq_net_a, tick_idx_b, liq_gross_b, liq_net_b, net_diff
            );
            // ===== END DEBUG =====
            if net_diff == 0 {
                println!(
                    "[DEBUG] Skipping tick pair ({}, {}) due to ZERO_NET",
                    tick_idx_a, tick_idx_b
                );
                continue;
            }
            let mock_ticks = vec![
                (tick_idx_a, liq_gross_a, liq_net_a),
                (tick_idx_b, liq_gross_b, liq_net_b),
            ];
            // ===== DEBUG: mock_ticks =====
            println!("[DEBUG] mock_ticks: {:?}", mock_ticks);
            // ===== END DEBUG =====
            // liquidityActive: 누적값이 아니라 해당 tick의 gross/net을 사용 (JS와 동일)
            let liquidity_active = liq_gross_a;
            let sqrt_price_x96 =
                TickMath::get_sqrt_ratio_at_tick(I24::try_from(tick_idx_a).unwrap())
                    .unwrap_or(U256::ZERO)
                    .to::<u128>();
            let tick_data_provider = TickListDataProvider::new(
                mock_ticks
                    .iter()
                    .map(|(tick, gross, net)| Tick {
                        index: *tick,
                        liquidity_gross: *gross,
                        liquidity_net: *net,
                    })
                    .collect(),
                tick_spacing,
            );
            let pool_sim = Pool::new_with_tick_data_provider(
                uni_token0.clone(),
                uni_token1.clone(),
                FeeAmount::try_from(fee).unwrap_or(FeeAmount::MEDIUM),
                U160::from(sqrt_price_x96),
                liquidity_active,
                tick_data_provider,
            )
            .unwrap();
            // swap 시뮬레이션 (JS와 동일)
            let next_sqrt_x96 =
                TickMath::get_sqrt_ratio_at_tick(I24::try_from(tick_idx_a - tick_spacing).unwrap())
                    .unwrap_or(U256::ZERO)
                    .to::<u128>();
            let max_amount_token0 =
                CurrencyAmount::from_raw_amount(uni_token0.clone(), u128::MAX).unwrap();
            if U160::from(next_sqrt_x96) >= U160::from(sqrt_price_x96) {
                continue;
            }
            let token1_amount = futures::executor::block_on(
                pool_sim.get_output_amount(&max_amount_token0, Some(U160::from(next_sqrt_x96))),
            )
            .unwrap();
            let price = Self::tick_to_price(tick_idx_a, token0.decimals, token1.decimals);
            let amount0_locked = token1_amount.numerator().to_f64().unwrap()
                / token1_amount.denominator().to_f64().unwrap()
                / price;
            let amount1_locked = token1_amount.numerator().to_f64().unwrap()
                / token1_amount.denominator().to_f64().unwrap();
            v3_levels.push(V3PriceLevel {
                tick_idx: tick_idx_a,
                price: Self::tick_to_price(tick_idx_a, token0.decimals, token1.decimals),
                tick_price: 1.0001_f64.powi(tick_idx_a),
                token0_liquidity: amount0_locked,
                token1_liquidity: amount1_locked,
                timestamp: Utc::now(),
            });
        }
        v3_levels.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        Ok(V3LiquidityDistribution {
            token0: token0.clone(),
            token1: token1.clone(),
            dex: self.name().to_lowercase(),
            chain_id: self.chain_id(),
            current_tick,
            price_levels: v3_levels,
            timestamp: Utc::now(),
        })
    }

    async fn get_all_pools_test(&self) -> Result<Vec<Pool>> {
        // This is only used for test mode in the indexer
        let pool_addresses = [
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
        ];
        let mut pools = Vec::new();
        for addr_str in pool_addresses.iter() {
            let addr_str = addr_str.trim();
            if addr_str.is_empty() {
                continue;
            }
            let pool_addr = match Address::from_str(addr_str) {
                Ok(a) => a,
                Err(_) => continue,
            };
            match self.get_pool(pool_addr).await {
                Ok(pool) => {
                    let _ = save_pool_async(self.storage.clone(), pool.clone()).await;
                    pools.push(pool)
                }
                Err(_) => {
                    let provider = self.provider.provider();
                    let pool_contract = IUniswapV3Pool::new(pool_addr, provider.clone());
                    let token0_addr = match pool_contract.token0().call().await {
                        Ok(a) => a,
                        Err(_) => continue,
                    };
                    let token1_addr = match pool_contract.token1().call().await {
                        Ok(a) => a,
                        Err(_) => continue,
                    };
                    let fee = match pool_contract.fee().call().await {
                        Ok(f) => f.to::<u64>(),
                        Err(_) => continue,
                    };
                    let tok0 = match self.fetch_or_load_token(token0_addr).await {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let tok1 = match self.fetch_or_load_token(token1_addr).await {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let pool = Pool {
                        address: pool_addr,
                        dex: self.name().into(),
                        chain_id: self.chain_id(),
                        tokens: vec![tok0, tok1],
                        creation_block: 0,
                        creation_timestamp: Utc::now(),
                        last_updated_block: 0,
                        last_updated_timestamp: Utc::now(),
                        fee,
                    };
                    let _ = save_pool_async(self.storage.clone(), pool.clone()).await;
                    pools.push(pool);
                }
            }
        }
        Ok(pools)
    }
}
