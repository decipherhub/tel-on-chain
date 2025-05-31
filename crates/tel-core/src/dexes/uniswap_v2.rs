use crate::dexes::DexProtocol;
use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, PriceLiquidity, Token};
use crate::providers::EthereumProvider;
use crate::storage::{save_pool_async, Storage};
use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tokio::try_join;

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

    // async fn fetch_or_load_token(&self, addr: Address) -> Result<Token, Error> {
    //     // 1) 이미 DB에 있으면 바로 반환
    //     if let Some(tok) = get_token_async(self.storage.clone(), addr, self.chain_id()).await? {
    //         return Ok(tok);
    //     }

    //     // 2) on-chain 메타데이터 조회
    //     let erc20 = IERC20Metadata::new(addr, self.provider.clone());
    //     let (symbol, name, decimals) = futures::future::try_join!(
    //         erc20.symbol().call(),
    //         erc20.name().call(),
    //         erc20.decimals().call()
    //     )?;

    //     let token = Token {
    //         address: addr,
    //         symbol,
    //         name,
    //         decimals: decimals as u8,
    //         chain_id: self.chain_id(),
    //     };

    //     // 3) DB에 저장
    //     save_token_async(self.storage.clone(), token.clone()).await?;
    //     Ok(token)
    // }

    // Helper method to get reserves from a pool - simplified version
    async fn get_reserves(&self, _pool_address: Address) -> Result<(u128, u128, u32), Error> {
        // This is a placeholder, in production we'd actually call the contract
        // Simplified for compatibility
        Ok((0, 0, 0))
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
        let token0 = Token {
            address: Address::ZERO,
            symbol: "DUMMY0".to_string(),
            name: "Dummy Token 0".to_string(),
            decimals: 18,
            chain_id: self.chain_id(),
        };

        let token1 = Token {
            address: Address::ZERO,
            symbol: "DUMMY1".to_string(),
            name: "Dummy Token 1".to_string(),
            decimals: 18,
            chain_id: self.chain_id(),
        };

        Ok(Pool {
            address: pool_address,
            dex: self.name().to_string(),
            chain_id: self.chain_id(),
            tokens: vec![token0, token1],
            creation_block: 0,
            creation_timestamp: Utc::now(),
            last_updated_block: 0,
            last_updated_timestamp: Utc::now(),
        })
    }

    async fn get_all_pools(&self) -> Result<Vec<Pool>, Error> {
        // 1. Alloy Provider (RootProvider<Ethereum>)
        let inner = self.provider.provider();

        // 2. Uniswap-V2 Factory
        let factory = IUniswapV2Factory::new(self.factory_address, inner.clone());

        // 3. 총 pair 수 (데모: 최대 10 개)
        let total: U256 = factory
            .allPairsLength()
            .call()
            .await
            .map_err(|e| Error::ProviderError(format!("allPairsLength: {e}")))?;

        let limit = std::cmp::min(total.to::<u64>(), 10) as usize;
        let mut pools = Vec::with_capacity(limit);

        // 4. 0 … limit-1 루프
        for i in 0..limit {
            // 4-a. pair 주소
            let pair_addr: Address = factory
                .allPairs(U256::from(i))
                .call()
                .await
                .map_err(|e| Error::ProviderError(format!("allPairs({i}): {e}")))?;

            // 4-b. pair 컨트랙트
            let pair = IUniswapV2Pair::new(pair_addr, inner.clone());

            // 4-c. token0 / token1 주소 ── 순차 호출 (수명 문제 無)
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

            // 4-d. Token stub & Pool 객체
            let stub = |addr| Token {
                address: addr,
                symbol: String::new(),
                name: String::new(),
                decimals: 0,
                chain_id: self.chain_id(),
            };

            let pool = Pool {
                address: pair_addr,
                dex: self.name().into(),
                chain_id: self.chain_id(),
                tokens: vec![stub(t0_addr), stub(t1_addr)],
                creation_block: 0,
                creation_timestamp: Utc::now(),
                last_updated_block: 0,
                last_updated_timestamp: Utc::now(),
            };

            // 4-e. DB 저장
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
        let price = if reserve0_float > 0.0 {
            reserve1_float / reserve0_float
        } else {
            0.0
        };

        // For Uniswap V2, there's just one price point (the current price)
        let price_level = PriceLiquidity {
            price,
            token0_liquidity: reserve0_float,
            token1_liquidity: reserve1_float,
            timestamp: Utc::now(),
        };

        Ok(LiquidityDistribution {
            token0: token0.clone(),
            token1: token1.clone(),
            dex: self.name().to_string(),
            chain_id: self.chain_id(),
            price_levels: vec![price_level],
            timestamp: Utc::now(),
        })
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
