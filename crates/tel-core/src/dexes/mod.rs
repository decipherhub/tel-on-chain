pub mod balancer;
pub mod curve;
pub mod sushiswap;
pub mod uniswap_v2;
pub mod uniswap_v3;
pub mod utils;

use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, Token};
use crate::providers::EthereumProvider;
use alloy_primitives::Address;
use async_trait::async_trait;
use std::sync::Arc;

/// Common interface for all DEX implementations
#[async_trait]
pub trait DexProtocol: Send + Sync {
    /// Get the name of the DEX
    fn name(&self) -> &str;

    /// Get the chain ID this DEX instance is operating on
    fn chain_id(&self) -> u64;

    /// Get the factory address for this DEX
    fn factory_address(&self) -> Address;

    /// Get the provider for this DEX
    fn provider(&self) -> Arc<EthereumProvider>;

    /// Get pool details for a specific pool address
    async fn get_pool(&self, pool_address: Address) -> Result<Pool, Error>;

    /// Get all pools
    async fn get_all_pools(&self) -> Result<Vec<Pool>, Error>;

    /// Get token details for a specific token address
    async fn get_token(&self, token_address: Address) -> Result<Token, Error> {
        // Default implementation uses the shared utils implementation
        utils::get_token(self.provider(), token_address, self.chain_id()).await
    }

    /// Get the liquidity distribution for a specific pool
    async fn get_liquidity_distribution(
        &self,
        pool_address: Address,
    ) -> Result<LiquidityDistribution, Error>;

    /// Calculate how a swap would impact prices
    async fn calculate_swap_impact(
        &self,
        pool_address: Address,
        token_in: Address,
        amount_in: f64,
    ) -> Result<f64, Error>;
}

pub fn get_dex_by_name(
    name: &str,
    provider: Arc<EthereumProvider>,
    factory_address: Address,
) -> Option<Box<dyn DexProtocol>> {
    match name {
        "uniswap_v2" => Some(Box::new(uniswap_v2::UniswapV2::new(
            provider,
            factory_address,
        ))),
        "uniswap_v3" => Some(Box::new(uniswap_v3::UniswapV3::new(
            provider,
            factory_address,
        ))),
        "sushiswap" => Some(Box::new(sushiswap::Sushiswap::new(
            provider,
            factory_address,
        ))),
        // Others will be implemented later
        _ => None,
    }
}
