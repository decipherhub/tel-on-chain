use crate::dexes::DexProtocol;
use crate::error::Error as TelError;
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
        if let Some(tok) =
            get_token_async(self.storage.clone(), addr, DexProtocol::chain_id(self)).await?
        {
            return Ok(tok);
        }
        let erc20 = IERC20Metadata::new(addr, self.provider.provider());
        let symbol = erc20
            .symbol()
            .call()
            .await
            .map_err(|e| TelError::ProviderError(format!("{e}")))?;
        let name = erc20
            .name()
            .call()
            .await
            .map_err(|e| TelError::ProviderError(format!("{e}")))?;
        let decimals = erc20
            .decimals()
            .call()
            .await
            .map_err(|e| TelError::ProviderError(format!("{e}")))?;
        let token = Token {
            address: addr,
            symbol,
            name,
            decimals: decimals as u8,
            chain_id: DexProtocol::chain_id(self),
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
            .map_err(|e| TelError::ProviderError(format!("get_logs: {}", e)))
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

    /// Calculates the amount of token0 / token1 that is effectively locked
    /// in the **active** tick range (`tick_idx .. tick_idx + tick_spacing`).
    async fn calculate_active_range_tokens_locked(
        &self,
        tick_idx: i32,
        liq_gross: u128,
        liq_net: i128, // may be 0 for active range
        tick_spacing: i32,
        fee: u32,
        slot0_sqrt_price_x96: U256,
        uni_token0: &uniswap_sdk_core::prelude::Token,
        uni_token1: &uniswap_sdk_core::prelude::Token,
    ) -> std::result::Result<V3PriceLevel, crate::Error> {
        use uniswap_v3_sdk::prelude::*;
        use uniswap_v3_sdk::utils::price_tick_conversions::tick_to_price;

        // -----------------------------------------------------------------
        // ① mock_ticks: lower = +liq_net , upper = -liq_net
        //    (총합 0, validate_list 통과)
        // -----------------------------------------------------------------
        let lower_idx = tick_idx;
        let upper_idx = tick_idx + tick_spacing;

        let mock_ticks = vec![
            Tick {
                index: I24::try_from(lower_idx).unwrap(),
                liquidity_gross: liq_gross,
                liquidity_net: liq_net, // +L  (range enters)
            },
            Tick {
                index: I24::try_from(upper_idx).unwrap(),
                liquidity_gross: liq_gross,
                liquidity_net: -liq_net, // -L  (range exits)
            },
        ];

        // -----------------------------------------------------------------
        // ② Pool simulator over just this active range
        // -----------------------------------------------------------------
        let pool_sim = Pool::new_with_tick_data_provider(
            uni_token0.clone(),
            uni_token1.clone(),
            FeeAmount::try_from(fee).unwrap_or(FeeAmount::MEDIUM),
            U160::from(slot0_sqrt_price_x96.to::<u128>()),
            liq_gross, // current active liquidity
            TickListDataProvider::new(mock_ticks.clone(), I24::try_from(tick_spacing).unwrap()),
        )
        .map_err(|e| crate::Error::ProviderError(format!("Pool: {e}")))?;

        // -----------------------------------------------------------------
        // ③ bottom‑of‑range calculation (swap token0 → token1)
        // -----------------------------------------------------------------
        let bottom_sqrt_x96 = {
            let sqrt: U256 = TickMath::get_sqrt_ratio_at_tick(I24::try_from(lower_idx).unwrap())
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
            I24::try_from(lower_idx).unwrap(),
        )
        .map_err(|e| crate::Error::ProviderError(format!("tick_to_price: {e}")))?;

        let amount0_locked = match price.invert().quote(&token1_amount) {
            Ok(q) => q.to_exact().parse::<f64>().unwrap_or(0.0),
            Err(e) => return Err(crate::Error::ProviderError(format!("quote error: {e}"))),
        };

        // -----------------------------------------------------------------
        // ④ top‑of‑range calculation (swap token1 → token0)
        // -----------------------------------------------------------------
        let top_sqrt_x96 = {
            let sqrt: U256 = TickMath::get_sqrt_ratio_at_tick(I24::try_from(upper_idx).unwrap())
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

        // -----------------------------------------------------------------
        // ⑤ 결과 구조체
        // -----------------------------------------------------------------
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
            Ok(None) => Err(crate::Error::DexError(format!(
                "Pool not found: {}",
                pool_address
            ))),
            Err(e) => Err(e),
        }
    }

    async fn get_all_pools(&self) -> Result<Vec<Pool>> {
        self.get_all_pools_test().await
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

    /// Return per-tick liquidity distribution identical to Uniswap-Interface chart
    async fn get_v3_liquidity_distribution(
        &self,
        pool_address: Address,
    ) -> Result<V3LiquidityDistribution> {
        use uniswap_v3_sdk::prelude::*;
        use uniswap_v3_sdk::utils::price_tick_conversions::tick_to_price;

        /* ── ① on-chain state ───────────────────────────────────────────── */
        let pool = self.get_pool(pool_address).await?;
        let token0 = &pool.tokens[0];
        let token1 = &pool.tokens[1];
        let feeTier = pool.fee as u32;

        let pool_c = IUniswapV3Pool::new(pool_address, self.provider.provider());

        let slot0 = pool_c
            .slot0()
            .call()
            .await
            .map_err(|e| TelError::ProviderError(format!("slot0: {e}")))?;
        let currentTick: i32 = slot0.tick.try_into().unwrap_or(0);
        let sqrtPriceX96_cur: u128 = slot0.sqrtPriceX96.to::<u128>();

        let tickSpacing: i32 = pool_c
            .tickSpacing()
            .call()
            .await
            .map_err(|e| crate::Error::ProviderError(format!("tickSpacing: {e}")))?
            .try_into()
            .unwrap_or(1);
        let liquidityActive: u128 = pool_c
            .liquidity()
            .call()
            .await
            .map_err(|e| crate::Error::ProviderError(format!("liquidity: {e}")))?
            .try_into()
            .unwrap_or(0);

        /* ── ② populated ticks (lens) + active-range supplement ───────────────── */
        let (_, mut chain_ticks) = self.get_active_ticks(pool_address).await?;
        chain_ticks.sort_by_key(|(t, _, _)| *t);

        let active_lower_tick = (currentTick / tickSpacing) * tickSpacing;
        if !chain_ticks.iter().any(|(t, _, _)| *t == active_lower_tick) {
            chain_ticks.push((active_lower_tick, liquidityActive, 0));
            chain_ticks.sort_by_key(|(t, _, _)| *t);
        }

        /* ── ③ SDK tokens ──────────────────────────────────────────────── */
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

        #[allow(clippy::too_many_arguments)]
        async fn build_bar(
            token0: &uniswap_sdk_core::prelude::Token,
            token1: &uniswap_sdk_core::prelude::Token,
            fee_tier: u32,
            lower_tick: i32,
            tick_spacing: i32,
            active_liquidity: u128, // active L at slot0
            gross_onchain: u128,
            net_onchain: i128,
            current_tick: i32,
            sqrt_price_x96_cur: u128,
        ) -> Result<(V3PriceLevel, i128)> {
            /* 1. mock-ticks (gross = |net|, interface rule) */
            let gross_effective = gross_onchain.max(net_onchain.unsigned_abs());
            let upper_tick = lower_tick + tick_spacing;

            let tick_provider = TickListDataProvider::new(
                vec![
                    Tick {
                        index: I24::try_from(lower_tick)
                            .map_err(|e| crate::Error::ProviderError(format!("I24 conv: {e}")))?,
                        liquidity_gross: gross_effective,
                        liquidity_net: net_onchain,
                    },
                    Tick {
                        index: I24::try_from(upper_tick)
                            .map_err(|e| crate::Error::ProviderError(format!("I24 conv: {e}")))?,
                        liquidity_gross: gross_effective,
                        liquidity_net: -net_onchain,
                    },
                ],
                I24::try_from(tick_spacing)
                    .map_err(|e| crate::Error::ProviderError(format!("I24 conv: {e}")))?,
            );

            /* 2. √P boundary values */
            let sqrt_lower: u128 = {
                let result: std::result::Result<U256, crate::Error> =
                    TickMath::get_sqrt_ratio_at_tick(I24::try_from(lower_tick).unwrap())
                        .map_err(|e| crate::Error::ProviderError(format!("TickMath: {e}")));
                result?.to::<u128>()
            };
            let sqrt_upper: u128 = {
                let result: std::result::Result<U256, crate::Error> =
                    TickMath::get_sqrt_ratio_at_tick(I24::try_from(upper_tick).unwrap())
                        .map_err(|e| crate::Error::ProviderError(format!("TickMath: {e}")));
                result?.to::<u128>()
            };

            /* 3. simulation start √P – always slot0 */
            let liquidity_for_sim = if (lower_tick..upper_tick).contains(&current_tick) {
                active_liquidity
            } else {
                net_onchain.unsigned_abs()
            };
            let mut pool_sim = Pool::new_with_tick_data_provider(
                token0.clone(),
                token1.clone(),
                FeeAmount::try_from(fee_tier).unwrap_or(FeeAmount::MEDIUM),
                U160::from(sqrt_price_x96_cur),
                liquidity_for_sim,
                tick_provider.clone(),
            )?;

            /* ── 4-A. move up one tick (price ↑, token1 in → token0 out, zero_for_one = false) */
            let limit_up = (sqrt_upper - 1).clamp(sqrt_price_x96_cur + 1, u128::MAX - 1);
            let max_token1_in = CurrencyAmount::from_raw_amount(token1.clone(), u128::MAX)?;
            let token0_out_up = pool_sim
                .get_output_amount(&max_token1_in, Some(U160::from(limit_up)))
                .await
                .unwrap_or_else(|_| CurrencyAmount::from_raw_amount(token0.clone(), 0).unwrap());

            /* ── 4-B. move down one tick (price ↓, token0 in → token1 out, zero_for_one = true) */
            let token1_out_down = if lower_tick > MIN_TICK.try_into().unwrap_or(i32::MIN) {
                let next_lower_tick = lower_tick - tick_spacing;
                let raw = {
                    let result: std::result::Result<U256, crate::Error> =
                        TickMath::get_sqrt_ratio_at_tick(I24::try_from(next_lower_tick).unwrap())
                            .map_err(|e| crate::Error::ProviderError(format!("TickMath: {e}")));
                    result?.to::<u128>() + 1
                };
                let limit_down = raw.clamp(1, sqrt_price_x96_cur - 1);

                let max_token0_in = CurrencyAmount::from_raw_amount(token0.clone(), u128::MAX)?;
                pool_sim
                    .get_output_amount(&max_token0_in, Some(U160::from(limit_down)))
                    .await
                    .unwrap_or_else(|_| CurrencyAmount::from_raw_amount(token1.clone(), 0).unwrap())
            } else {
                CurrencyAmount::from_raw_amount(token1.clone(), 0).unwrap()
            };

            /* 5. convert amounts to token units */
            let price_lower = tick_to_price(
                token0.clone(),
                token1.clone(),
                I24::try_from(lower_tick).unwrap(),
            )?;

            let token0_needed_up = price_lower
                .quote(&token0_out_up)?
                .to_exact()
                .parse::<f64>()
                .unwrap_or(0.0);

            let token1_needed_down = price_lower
                .invert()
                .quote(&token1_out_down)?
                .to_exact()
                .parse::<f64>()
                .unwrap_or(0.0);

            /* 6. column placement */
            let (token0_liq, token1_liq) = if (lower_tick..upper_tick).contains(&current_tick) {
                (token0_needed_up, token1_needed_down) // current bar
            } else if lower_tick > current_tick {
                (token0_needed_up, 0.0) // upper bar
            } else {
                (0.0, token1_needed_down) // lower bar
            };

            /* 7. return result */
            Ok((
                V3PriceLevel {
                    tick_idx: lower_tick,
                    price: price_lower
                        .to_significant(12, Some(Rounding::RoundDown))?
                        .parse::<f64>()
                        .unwrap_or(0.0),
                    tick_price: 1.0001_f64.powi(lower_tick),
                    token0_liquidity: token0_liq,
                    token1_liquidity: token1_liq,
                    timestamp: chrono::Utc::now(),
                },
                net_onchain, // for running_L update
            ))
        }

        /* ── ④ create bars for all populated ticks ───────────────────── */
        let mut bars: Vec<V3PriceLevel> = Vec::with_capacity(chain_ticks.len());
        let mut running_L: i128 = liquidityActive as i128;

        // current tick included ↑ direction
        for (tick, lg, ln) in chain_ticks.iter().filter(|(t, _, _)| *t >= currentTick) {
            let (bar, net_delta) = build_bar(
                &uni_token0,
                &uni_token1,
                feeTier,
                *tick,
                tickSpacing,
                running_L.max(0) as u128,
                *lg,
                *ln,
                currentTick,
                sqrtPriceX96_cur,
            )
            .await?;
            bars.push(bar);
            running_L += net_delta as i128;
        }
        // ↓ direction
        running_L = liquidityActive as i128;
        for (tick, lg, ln) in chain_ticks
            .iter()
            .rev()
            .filter(|(t, _, _)| *t < currentTick)
        {
            running_L -= *ln as i128;
            let (bar, _) = build_bar(
                &uni_token0,
                &uni_token1,
                feeTier,
                *tick,
                tickSpacing,
                running_L.max(0) as u128,
                *lg,
                *ln,
                currentTick,
                sqrtPriceX96_cur,
            )
            .await?;
            bars.push(bar);
        }

        /* ── ⑤ sort by price & TVL-shift(offset) ─────────────────────────── */
        bars.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap());
        for i in 1..bars.len() {
            bars[i - 1].token0_liquidity = bars[i].token0_liquidity;
            bars[i - 1].token1_liquidity = bars[i].token1_liquidity;
        }

        /* ── ⑥ return ──────────────────────────────────────────────────── */
        Ok(V3LiquidityDistribution {
            token0: token0.clone(),
            token1: token1.clone(),
            dex: self.name().into(),
            chain_id: self.chain_id(),
            current_tick: currentTick,
            price_levels: bars,
            timestamp: chrono::Utc::now(),
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
                        chain_id: DexProtocol::chain_id(self),
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
