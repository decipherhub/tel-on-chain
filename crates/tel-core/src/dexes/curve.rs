use alloy_primitives::Address;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    dexes::DexProtocol,
    models::{LiquidityDistribution, Pool},
    providers::{EthereumProvider, ProviderManager},
    Error, Result,
};

pub struct Curve {
    factory_address: Address,
    provider: Arc<EthereumProvider>,
}

impl Curve {
    pub fn new(
        factory_address: Address,
        provider_manager: Arc<ProviderManager>,
        chain_id: u64,
    ) -> Result<Self> {
        let provider = provider_manager.by_chain_id(chain_id).ok_or_else(|| {
            Error::ProviderError(format!("No provider found for chain {}", chain_id))
        })?;

        Ok(Self {
            factory_address,
            provider,
        })
    }
}

#[async_trait]
impl DexProtocol for Curve {
    fn name(&self) -> &str {
        "curve"
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

    async fn get_pool(&self, _pool_address: Address) -> Result<Pool> {
        // TODO: Implement
        Err(Error::NotImplemented)
    }

    async fn get_all_pools(&self) -> Result<Vec<Pool>> {
        // TODO: Implement
        Ok(vec![])
    }

    async fn get_liquidity_distribution(
        &self,
        _pool_address: Address,
    ) -> Result<LiquidityDistribution> {
        // TODO: Implement
        Err(Error::NotImplemented)
    }

    async fn calculate_swap_impact(
        &self,
        _pool_address: Address,
        _token_in: Address,
        _amount_in: f64,
    ) -> Result<f64> {
        // TODO: Implement
        Err(Error::NotImplemented)
    }
}
