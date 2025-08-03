use crate::dexes::DexProtocol;
use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, PriceLiquidity, Side, Token};
use crate::providers::EthereumProvider;
use crate::storage::{
    get_pool_async, get_token_async, save_liquidity_distribution_async, save_pool_async,
    save_token_async, Storage,
};
use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

sol! {
    // ── Uniswap V2 Factory ───────────────────────────────────────────
    #[sol(rpc)]
    interface IUniswapV2Factory {
        function allPairsLength() external view returns (uint256);
        function allPairs(uint256) external view returns (address);
    }

    // ── Uniswap V2 Pair ──────────────────────────────────────────────
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function token0() external view returns (address);
        function token1() external view returns (address);
    }

    #[sol(rpc)]
    interface IERC20Metadata {
        function symbol()   external view returns (string);
        function name()     external view returns (string);
        function decimals() external view returns (uint8);
    }
}

pub struct UniswapV2 {
    provider: Arc<EthereumProvider>,
    factory_address: Address,
    storage: Arc<dyn Storage>,
}

impl UniswapV2 {
    /// Creates a new UniswapV2 instance with the specified Ethereum provider, factory contract address, and storage backend.
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

    async fn fetch_or_load_token(&self, addr: Address) -> Result<Token, Error> {
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


    async fn get_reserves(&self, _pool_address: Address) -> Result<(u128, u128, u32), Error> {
        // This is a placeholder, in production we'd actually call the contract
        // Simplified for compatibility
        // NO reserves store in DB?
        let inner = self.provider.provider();
        let pair = IUniswapV2Pair::new(_pool_address, inner.clone());
        let get_reserves_return = pair
            .getReserves()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("getReserves: {e}")))?;
        let (reserve0, reserve1, last_updated_timestamp) = (
            get_reserves_return.reserve0,
            get_reserves_return.reserve1,
            get_reserves_return.blockTimestampLast,
        );
        let reserve0 = reserve0.to::<u128>();
        let reserve1 = reserve1.to::<u128>();
        Ok((reserve0, reserve1, last_updated_timestamp))
    }

    fn build_cumulative_price_levels(
        reserves: (u128, u128),
    ) -> Vec<PriceLiquidity> {
        let current_price = reserves.1 as f64 / reserves.0 as f64;
    
        (-50..=100)
            .map(|i| {
                let factor   = 1.0 + i as f64 / 100.0;
                let sqrt_f   = factor.sqrt();
    
                // price up (f > 1) : token0 is sold and removed from pool
                // price down (f < 1) : token1 is sold and removed from pool
                let (liq0, liq1) = if factor >= 1.0 {
                    (
                        reserves.0 as f64 * (1.0 - 1.0 / sqrt_f),
                        0.0,
                    )
                } else {
                    (
                        0.0,
                        reserves.1 as f64 * (1.0 - sqrt_f),
                    )
                };
    
                PriceLiquidity {
                    side: if factor >= 1.0 { Side::Sell } else { Side::Buy },
                    lower_price: current_price * factor,
                    upper_price: current_price * factor,
                    token0_liquidity: liq0,
                    token1_liquidity: liq1,
                    timestamp: Utc::now(),
                }
            })
            .collect()
    }


    // Builds a visual representation of liquidity for a Uniswap V2 pool,
    // assuming uniform distribution across price buckets.
    // The total representative liquidity is divided equally among all buckets.
    fn build_uniform_liquidity_levels(
        reserves_float: (f64, f64),
        current_price: f64,
    ) -> Vec<PriceLiquidity> {
        if reserves_float.0 <= 0.0 || reserves_float.1 <= 0.0 {
            return vec![];
        }

        // 1. Calculate the total representative liquidity (L = sqrt(x*y)).
        let total_representative_liquidity = (reserves_float.0 * reserves_float.1).sqrt();

        // 2. Define the price range and count the number of buckets.
        let price_range = -50..=50;
        let num_buckets = price_range.clone().count() as f64;
        if num_buckets == 0.0 {
            return vec![];
        }

        // 3. Divide the total liquidity by the number of buckets to get per-bucket liquidity.
        let liquidity_per_bucket = total_representative_liquidity / num_buckets;

        // 4. Create the price buckets with the correctly scaled liquidity.
        price_range
            .map(|i| {
                let price_ratio = 1.0 + (i as f64 / 100.0);
                let price_level = current_price * price_ratio;

                let (token0_liq, token1_liq, side) = if i < 0 {
                    (0.0, liquidity_per_bucket, Side::Buy)
                } else if i > 0 {
                    (liquidity_per_bucket, 0.0, Side::Sell)
                } else {
                    // At the current price, show half on each side.
                    (liquidity_per_bucket / 2.0, liquidity_per_bucket / 2.0, Side::Sell)
                };

                PriceLiquidity {
                    side,
                    lower_price: price_level,
                    upper_price: price_level,
                    token0_liquidity: token0_liq,
                    token1_liquidity: token1_liq,
                    timestamp: Utc::now(),
                }
            })
            .collect()
    }
}

#[async_trait]
impl DexProtocol for UniswapV2 {
    fn name(&self) -> &str {
        "uniswap_v2"
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

    async fn get_pool(&self, pool_address: Address) -> Result<Pool, Error> {
        let inner = self.provider.provider();
        let pair = IUniswapV2Pair::new(pool_address, inner.clone());

        let t0_addr = pair
            .token0()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("token0(): {e}")))?;

        let t1_addr = pair
            .token1()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("token1(): {e}")))?;

        let token0 = self.fetch_or_load_token(t0_addr).await?;
        let token1 = self.fetch_or_load_token(t1_addr).await?;

        let pool = Pool {
            address: pool_address,
            dex: self.name().into(),
            chain_id: self.chain_id(),
            tokens: vec![token0, token1],
            creation_block: 0,
            creation_timestamp: Utc::now(),
            last_updated_block: 0,
            last_updated_timestamp: Utc::now(),
            fee: 3000, // 0.3% = 3000 (UniswapV2 standard)
        };

        save_pool_async(self.storage.clone(), pool.clone()).await?;
        Ok(pool)
    }

    
    async fn get_all_pools(&self) -> Result<Vec<Pool>, Error> {
        // 1. Alloy Provider (RootProvider<Ethereum>)
        let inner = self.provider.provider();

        // 2. Uniswap-V2 Factory
        let factory = IUniswapV2Factory::new(self.factory_address, inner.clone());

        // 3. Total pair count (demo: max 10)
        let total: U256 = factory
            .allPairsLength()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("allPairsLength: {e}")))?;

        let limit = std::cmp::min(total.to::<u64>(), 10) as usize;
        let mut pools = Vec::with_capacity(limit);

        // 4. Loop 0 … limit-1
        for i in 0..limit {
            // 4-a. pair address
            let pair_addr: Address = factory
                .allPairs(U256::from(i))
                .call()
                .await
                .map_err(|e| Error::ProviderError(format!("allPairs({i}): {e}")))?;

            // 4-b. pair contract
            let pair = IUniswapV2Pair::new(pair_addr, inner.clone());

            // 4-c. token0 / token1 address -- sequential call (no naming issue)
            let t0_addr = pair
                .token0()
                .call()
                .await
                .map_err(|e| Error::ProviderError(format!("token0(): {e}")))?;

            let t1_addr = pair
                .token1()
                .call()
                .await
                .map_err(|e| Error::ProviderError(format!("token1(): {e}")))?;

            // 4-d. Fetch actual token metadata

            let token0 = self.fetch_or_load_token(t0_addr).await?;
            let token1 = self.fetch_or_load_token(t1_addr).await?;

            let pool = Pool {
                address: pair_addr,
                dex: self.name().into(),
                chain_id: self.chain_id(),
                tokens: vec![token0, token1],
                creation_block: 0,
                creation_timestamp: Utc::now(),
                last_updated_block: 0,
                last_updated_timestamp: Utc::now(),
                fee: 3000, // 0.3% = 3000 (UniswapV2 standard)
            };

            // 4-e. Save to DB
            save_pool_async(self.storage.clone(), pool.clone()).await?;
            pools.push(pool);
        }

        Ok(pools)
    }

    
    async fn get_liquidity_distribution(
        &self,
        pool_address: Address,
    ) -> Result<LiquidityDistribution, Error> {
        let pool = self.get_pool(pool_address).await?;
        let (reserve0, reserve1, _) = self.get_reserves(pool_address).await?;

        let token0 = &pool.tokens[0];
        let token1 = &pool.tokens[1];

        // Convert reserves to float for price calculation
        let reserve0_float = reserve0 as f64 / 10f64.powi(token0.decimals as i32);
        let reserve1_float = reserve1 as f64 / 10f64.powi(token1.decimals as i32);

        // Calculate price (token1/token0)
        let current_price = if reserve0_float > 0.0 {
            reserve1_float / reserve0_float
        } else {
            0.0
        };

        let price_levels = Self::build_uniform_liquidity_levels((reserve0_float, reserve1_float), current_price);

        let distribution = LiquidityDistribution {
            current_price: current_price,
            token0: token0.clone(),
            token1: token1.clone(),
            dex: self.name().to_string(),
            chain_id: self.chain_id(),
            price_levels: price_levels, // Use the new levels directly
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
    ) -> Result<f64, Error> {
        // Simplified placeholder implementation
        Ok(0.0)
    }
}
