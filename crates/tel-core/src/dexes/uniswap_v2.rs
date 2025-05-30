use crate::dexes::DexProtocol;
use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, PriceLiquidity, Token};
use crate::providers::EthereumProvider;
use crate::storage::{Storage, SqliteStorage};
use alloy_primitives::Address;
use alloy_sol_types::sol;
use async_trait::async_trait;
use chrono::Utc;
use IUniswapV2Pair::getReservesReturn;
use std::sync::Arc;

sol! {
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }
}

pub struct UniswapV2 {
    provider: Arc<EthereumProvider>,
    factory_address: Address,
    storage: Arc<dyn Storage>,
}

impl UniswapV2 {
    pub fn new(provider: Arc<EthereumProvider>, factory_address: Address) -> Self {
        let storage = SqliteStorage::new("tel_on_chain.db")
            .expect("Failed to initialize SqliteStorage");
        let storage = Arc::new(storage);
        Self {
            provider,
            factory_address,
            storage,
        }
    }

    // Helper method to get reserves from a pool - simplified version
    async fn get_reserves(&self, pool_address: Address) -> Result<(u128, u128, u32), Error> {
        // 1) Pair 컨트랙트 인터페이스 생성
        let contract = IUniswapV2Pair::new(pool_address, self.provider.provider());
        let result: getReservesReturn = contract.getReserves()
            .call()
            .await?;
        // let (reserve0, reserve1, block_timestamp_last) = 
        // contract.getReserves()
        // .call()
        // .await?;
        Ok((reserve0 as u128, reserve1 as u128, block_timestamp_last))
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

    async fn get_pool(&self, pool_address: Address) -> Result<Pool, Error> {
        // This is a placeholder implementation
        // In production, we'd use provider.call() with correct parameters

        // For simplicity, creating a dummy pool
        // let token0 = Token {
        //     address: Address::ZERO,
        //     symbol: "DUMMY0".to_string(),
        //     name: "Dummy Token 0".to_string(),
        //     decimals: 18,
        //     chain_id: self.chain_id(),
        // };

        // let token1 = Token {
        //     address: Address::ZERO,
        //     symbol: "DUMMY1".to_string(),
        //     name: "Dummy Token 1".to_string(),
        //     decimals: 18,
        //     chain_id: self.chain_id(),
        // };
        self.storage.get_pool(pool_address)
            .map_err(|_| Error::DatabaseError(pool_address.to_string()));
        
    }

    async fn get_all_pools(&self) -> Result<Vec<Pool>, Error> {
        // This would require scanning events or getting pools from an indexer
        // For simplicity, returning empty vec
        Ok(Vec::new())
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
        let price = if reserve0_float > 0.0 {
            reserve1_float / reserve0_float
        } else {
            0.0
        };

        let num_buckets = 20;
        let bucket_width = 0.01;
        let mid = num_buckets as f64 / 2.0;
        let mut price_levels = Vec::with_capacity(num_buckets);

        // For Uniswap V2, there's just one price point (the current price)
        // let price_level = PriceLiquidity {
        //     price,
        //     token0_liquidity: reserve0_float,
        //     token1_liquidity: reserve1_float,
        //     timestamp: Utc::now(),
        // };
        for i in 0..num_buckets{
            let mid_price = price * (1.0 + i as f64 - mid * bucket_width);
            let (liquidity0, liquidity1) = if(mid_price - price).abs() <= (bucket_width/2.0) {
                (reserve0_float, reserve1_float)
            } else {
                (0.0, 0.0)
            };
            price_levels.push(PriceLiquidity{
                price: mid_price,
                token0_liquidity:liquidity0,
                token1_liquidity:liquidity1,
                timestamp: Utc::now(),
            });
        }

        

        Ok(LiquidityDistribution {
            token0: token0.clone(),
            token1: token1.clone(),
            dex_name: self.name().to_string(),
            chain_id: self.chain_id(),
            price_levels,
            timestamp: Utc::now(),
        })
    }

    async fn calculate_swap_impact(
        &self,
        pool_address: Address,
        token_in: Address,
        amount_in: f64,
    ) -> Result<f64, Error> {
        // Simplified placeholder implementation
        //Ok(0.0)
        let (reserve0, reserve1, _) = self.get_reserves(pool_address).await?;
        let r0 = reserve0 as f64;
        let r1 = reserve1 as f64;
        if r0 == 0.0 { return Ok(0.0); }

        // 원본 가격 계산
        let original_price = r1 / r0;

        // 상수곱 곡선 시뮬레이션
        let pool = self.get_pool(pool_address).await?;
        let is_token0 = token_in == pool.tokens[0].address;

        let (new_r0, new_r1) = if is_token0 {
            let dx = amount_in;
            let dy = r1 - (r0 * r1) / (r0 + dx);
            (r0 + dx, r1 - dy)
        } else {
            let dx = amount_in;
            let dy = r0 - (r0 * r1) / (r1 + dx);
            (r0 - dy, r1 + dx)
        };

        // 새로운 가격 및 임팩트 계산
        if new_r0 == 0.0 { return Ok(0.0); }
        let new_price = new_r1 / new_r0;
        let impact_pct = (new_price / original_price) - 1.0;

        Ok(impact_pct)
    }
    
    async fn get_token(&self, token_address: Address) -> Result<Token, Error> {
        // Default implementation uses the shared utils implementation
        super::utils::get_token(self.provider(), token_address, self.chain_id()).await
    }
}
