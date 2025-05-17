pub mod balancer;
pub mod curve;
pub mod sushiswap;
pub mod uniswap_v2;
pub mod uniswap_v3;

use crate::error::Error;
use crate::models::{LiquidityDistribution, Pool, Token};
use crate::providers::EthereumProvider;
use alloy_primitives::Address;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Common interface for all DEX implementations
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
    fn get_pool<'a>(
        &'a self,
        pool_address: Address,
    ) -> Pin<Box<dyn Future<Output = Result<Pool, Error>> + Send + 'a>>;

    /// Get all pools for a specific token
    fn get_pools_for_token<'a>(
        &'a self,
        token_address: Address,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Pool>, Error>> + Send + 'a>>;

    /// Get token details for a specific token address
    fn get_token<'a>(
        &'a self,
        token_address: Address,
    ) -> Pin<Box<dyn Future<Output = Result<Token, Error>> + Send + 'a>>;

    /// Get the liquidity distribution for a specific pool
    fn get_liquidity_distribution<'a>(
        &'a self,
        pool_address: Address,
    ) -> Pin<Box<dyn Future<Output = Result<LiquidityDistribution, Error>> + Send + 'a>>;

    /// Calculate how a swap would impact prices
    fn calculate_swap_impact<'a>(
        &'a self,
        pool_address: Address,
        token_in: Address,
        amount_in: f64,
    ) -> Pin<Box<dyn Future<Output = Result<f64, Error>> + Send + 'a>>;
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
