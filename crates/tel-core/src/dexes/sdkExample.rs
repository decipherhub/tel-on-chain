// use { FeeAmount, Pool as PoolV3, TICK_SPACINGS, TickMath as TickMathV3, tickToPrice } from '@uniswap/v3-sdk'

// use { Currency, CurrencyAmount, Token } from '@uniswap/sdk-core'

// pub fn get_v3_liquidity_distribution(){
//     // v3 sdk -> pool.rs
//     pub fn new_with_tick_data_provider(
//         token_a: Token,
//         token_b: Token,
//         fee: FeeAmount,
//         sqrt_ratio_x96: U160,
//         liquidity: u128,
//         tick_data_provider: TP,
//     ) -> Result<Self, Error> {
//         let (token0, token1) = if token_a.sorts_before(&token_b)? {
//             (token_a, token_b)
//         } else {
//             (token_b, token_a)
//         };
//         Ok(Self {
//             token0,
//             token1,
//             fee,
//             sqrt_ratio_x96,
//             liquidity,
//             tick_current: TP::Index::from_i24(sqrt_ratio_x96.get_tick_at_sqrt_ratio()?),
//             tick_data_provider,
//         })
//     }

//     // v3 sdk -> tick_data_provider.rs
//     impl<TP> TickDataProvider for TP
// where
//     TP: Deref<Target: TickDataProvider> + Send + Sync,
// {
//     type Index = <<TP as Deref>::Target as TickDataProvider>::Index;

//     #[inline]
//     async fn get_tick(&self, index: Self::Index) -> Result<Tick<Self::Index>, Error> {
//         self.deref().get_tick(index).await
//     }

//     #[inline]
//     async fn next_initialized_tick_within_one_word(
//         &self,
//         tick: Self::Index,
//         lte: bool,
//         tick_spacing: Self::Index,
//     ) -> Result<(Self::Index, bool), Error> {
//         self.deref()
//             .next_initialized_tick_within_one_word(tick, lte, tick_spacing)
//             .await
//     }
// }

//     pool.tick_spacing() // v3 sdk -> pool.rs

//     const liqGross = JSBI.greaterThan(tick.liquidityNet, JSBI.BigInt(0))
//       ? tick.liquidityNet
//       : JSBI.multiply(tick.liquidityNet, JSBI.BigInt('-1'))

//     // v3 sdk -> tick_math.rs
//       pub trait TickMath: Sized {
//         fn get_sqrt_ratio_at_tick(tick: I24) -> Result<Self, Error>;
//         fn get_tick_at_sqrt_ratio(self) -> Result<I24, Error>;
//     }

//     // v3 sdk -> pool.rs
//     pub fn new_with_tick_data_provider(
//         token_a: Token,
//         token_b: Token,
//         fee: FeeAmount,
//         sqrt_ratio_x96: U160,
//         liquidity: u128,
//         tick_data_provider: TP,
//     ) -> Result<Self, Error> {
//         let (token0, token1) = if token_a.sorts_before(&token_b)? {
//             (token_a, token_b)
//         } else {
//             (token_b, token_a)
//         };
//         Ok(Self {
//             token0,
//             token1,
//             fee,
//             sqrt_ratio_x96,
//             liquidity,
//             tick_current: TP::Index::from_i24(sqrt_ratio_x96.get_tick_at_sqrt_ratio()?),
//             tick_data_provider,
//         })
//     }

//     const mockTicks = [
//       {
//         index: tick.tick,
//         liquidityGross: liqGross,
//         liquidityNet: JSBI.multiply(tick.liquidityNet, JSBI.BigInt('-1')),
//       },
//       {
//         index: tick.tick + TICK_SPACINGS[feeTier],
//         liquidityGross: liqGross,
//         liquidityNet: tick.liquidityNet,
//       },
//     ]

//     const pool = new PoolV3(token0, token1, Number(feeTier), sqrtPriceX96, tick.liquidityActive, tick.tick, mockTicks)

//     // v3 sdk -> tick_math.rs
//     const nextSqrtX96 = TickMathV3.getSqrtRatioAtTick(tick.tick - tickSpacing) //     fn get_sqrt_ratio_at_tick(tick: I24) -> Result<Self, Error>;

//     // uniswap sdk core -> currency_amount.rs
//     CurrencyAmount::from_raw_amount(self.output_currency().clone(), 0)?; //     pub fn from_raw_amount(currency: T, raw_amount: impl Into<BigInt>) -> Result<Self, Error> {

//     // v3 sdk -> pool.rs
//     const token1Amount = (await pool.getOutputAmount(maxAmountToken0, nextSqrtX96))[0] //pub async fn get_output_amount(
//         &self,
//         input_amount: &CurrencyAmount<impl BaseCurrency>,
//         sqrt_price_limit_x96: Option<U160>,
//     ) -> Result<CurrencyAmount<Token>, Error>

//     // v3 sdk -> price_tick_conversions.rs
//     pub fn tick_to_price(
//         base_token: Token,
//         quote_token: Token,
//         tick: I24,
//     ) -> Result<Price<Token, Token>, Error> {
//         let sqrt_ratio_x96 = get_sqrt_ratio_at_tick(tick)?;
//         let ratio_x192 = sqrt_ratio_x96.to_big_int().pow(2);
//         Ok(if base_token.sorts_before(&quote_token)? {
//             Price::new(base_token, quote_token, Q192_BIG_INT, ratio_x192)
//         } else {
//             Price::new(base_token, quote_token, ratio_x192, Q192_BIG_INT)
//         })
//     }
//     // sdk price is "tick_to_price"
//     const amount0Locked = parseFloat(tick.sdkPrice.invert().quote(token1Amount).toExact())
//     // sdk core -> currency_amount.rs -> to_exact()
//     const amount1Locked = parseFloat(token1Amount.toExact())

// }
