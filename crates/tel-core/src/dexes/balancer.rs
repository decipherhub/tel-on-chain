use alloy_primitives::Address;
use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    dexes::DexProtocol,
    models::{LiquidityDistribution, Pool},
    providers::{EthereumProvider, ProviderManager},
    Error, Result,
};

pub struct Balancer {
    factory_address: Address,
    provider: Arc<EthereumProvider>,
}

impl Balancer {
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

    pub async fn get_pool(&self, _pool_address: Address) -> Result<Pool> {
        // TODO: Implement
        Err(Error::NotImplemented)
    }

    pub async fn get_liquidity_distribution(
        &self,
        _pool_address: Address,
    ) -> Result<LiquidityDistribution> {
        // TODO: Implement
        Err(Error::NotImplemented)
    }

    pub async fn get_price_impact(
        &self,
        _pool_address: Address,
        _token_in: Address,
        _amount_in: f64,
    ) -> Result<f64> {
        // TODO: Implement
        Err(Error::NotImplemented)
    }
}

#[async_trait]
impl DexProtocol for Balancer {
    fn name(&self) -> &str {
        "balancer"
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
        // Placeholder
        Err(Error::Unknown("Not implemented".to_string()))
    }

    async fn get_all_pools(&self) -> Result<Vec<Pool>> {
        // Placeholder
        Ok(Vec::new())
    }

    async fn get_liquidity_distribution(
        &self,
        _pool_address: Address,
    ) -> Result<LiquidityDistribution> {
        // Placeholder
        Err(Error::Unknown("Not implemented".to_string()))
    }

    async fn calculate_swap_impact(
        &self,
        _pool_address: Address,
        _token_in: Address,
        _amount_in: f64,
    ) -> Result<f64> {
        // Placeholder
        Err(Error::Unknown("Not implemented".to_string()))
    }
}
