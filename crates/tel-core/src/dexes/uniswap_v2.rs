use crate::dexes::DexProtocol;
use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, PriceLiquidity, Token};
use crate::providers::EthereumProvider;
use crate::storage::{get_pool_async, save_liquidity_distribution_async, save_pool_async, Storage};
use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use tokio::try_join;
use IUniswapV2Pair::getReservesReturn;

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

    /// Retrieves the reserves and last update timestamp for a given pool address.
    ///
    /// This simplified placeholder returns zero values for reserves and timestamp.
    /// In production, this would query the pool contract for actual reserve data.
    ///
    /// # Arguments
    ///
    /// * `_pool_address` - The address of the liquidity pool to query.
    ///
    /// # Returns
    ///
    /// A tuple containing the reserves of token0, token1, and the last update timestamp.
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

    /// Retrieves information about a specific Uniswap V2 pool by its address.
    ///
    /// Returns a `Pool` object with placeholder token data. In production, this would fetch real pool and token metadata from the blockchain.
    ///
    /// # Arguments
    ///
    /// * `pool_address` - The address of the Uniswap V2 pool to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `Pool` object with dummy tokens, or an error if retrieval fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let pool = uniswap_v2.get_pool(Address::from_low_u64_be(0x1234)).await?;
    /// assert_eq!(pool.address, Address::from_low_u64_be(0x1234));
    /// ```
    async fn get_pool(&self, pool_address: Address) -> Result<Pool, Error> {
        let pool_result = get_pool_async(self.storage.clone(), pool_address).await;
        match pool_result {
            Ok(Some(pool)) => Ok(pool),
            Ok(None) => Err(Error::DexError(format!("Pool not found: {}", pool_address))),
            Err(e) => Err(e),
        }
    }

    /// Retrieves up to 10 Uniswap V2 pools from the factory contract and saves them to storage.
    ///
    /// Queries the Uniswap V2 factory contract for the total number of pairs, fetches up to 10 pool addresses and their associated token addresses, constructs pool objects with token stubs, saves each pool asynchronously to storage, and returns the list of pools.
    ///
    /// # Returns
    /// A vector of `Pool` objects representing the discovered Uniswap V2 pools.
    ///
    /// # Errors
    /// Returns an error if any on-chain contract call fails or if saving a pool to storage fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let pools = uniswap_v2.get_all_pools().await?;
    /// assert!(!pools.is_empty());
    /// ```
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
                fee: 3000, // 0.3% = 3000 (0.0001% 단위)
            };

            // 4-e. DB 저장
            save_pool_async(self.storage.clone(), pool.clone()).await?;
            pools.push(pool);
        }

        Ok(pools)
    }

    /// Retrieves the current liquidity distribution and price for a given Uniswap V2 pool.
    ///
    /// Calculates the price and available liquidity for both tokens in the specified pool,
    /// returning a `LiquidityDistribution` with a single price point representing the current state.
    ///
    /// # Parameters
    /// - `pool_address`: The address of the Uniswap V2 pool to query.
    ///
    /// # Returns
    /// A `LiquidityDistribution` containing token information, DEX name, chain ID, price levels, and timestamp.
    ///
    /// # Errors
    /// Returns an error if the pool or reserves cannot be retrieved.
    ///
    /// # Examples
    ///
    /// ```
    /// let distribution = uniswap_v2.get_liquidity_distribution(pool_address).await?;
    /// println!("Current price: {}", distribution.price_levels[0].price);
    /// ```
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
        let distribution = LiquidityDistribution {
            token0: token0.clone(),
            token1: token1.clone(),
            dex: self.name().to_string(),
            chain_id: self.chain_id(),
            price_levels: vec![price_level],
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
