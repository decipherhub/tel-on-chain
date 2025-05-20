use crate::config::RpcConfig;
use crate::error::Error;
use alloy_network::Ethereum;
use alloy_provider::RootProvider;
use reqwest::Url;
use std::sync::Arc;

/// A provider for interacting with an Ethereum node
pub struct EthereumProvider {
    provider: Arc<RootProvider<Ethereum>>,
    chain_id: u64,
}

impl EthereumProvider {
    /// Create a new Ethereum provider from the given configuration
    pub fn new(config: &RpcConfig, chain_id: u64) -> Result<Self, Error> {
        let url = config
            .url
            .parse::<Url>()
            .map_err(|e| Error::ProviderError(e.to_string()))?;

        // Create the provider with the URL
        let provider = Arc::new(RootProvider::<Ethereum>::new_http(url));

        Ok(Self { provider, chain_id })
    }

    /// Get the provider instance
    pub fn provider(&self) -> Arc<RootProvider<Ethereum>> {
        self.provider.clone()
    }

    /// Get the chain ID
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

/// ProviderManager handles multiple providers for different chains
pub struct ProviderManager {
    ethereum: Arc<EthereumProvider>,
    polygon: Option<Arc<EthereumProvider>>,
    arbitrum: Option<Arc<EthereumProvider>>,
    optimism: Option<Arc<EthereumProvider>>,
}

impl ProviderManager {
    /// Create a new provider manager from the given configurations
    pub fn new(
        eth_config: &RpcConfig,
        polygon_config: Option<&RpcConfig>,
        arbitrum_config: Option<&RpcConfig>,
        optimism_config: Option<&RpcConfig>,
    ) -> Result<Self, Error> {
        let ethereum = Arc::new(EthereumProvider::new(eth_config, 1)?);

        let polygon = match polygon_config {
            Some(config) => Some(Arc::new(EthereumProvider::new(config, 137)?)),
            None => None,
        };

        let arbitrum = match arbitrum_config {
            Some(config) => Some(Arc::new(EthereumProvider::new(config, 42161)?)),
            None => None,
        };

        let optimism = match optimism_config {
            Some(config) => Some(Arc::new(EthereumProvider::new(config, 10)?)),
            None => None,
        };

        Ok(Self {
            ethereum,
            polygon,
            arbitrum,
            optimism,
        })
    }

    /// Get the Ethereum provider
    pub fn ethereum(&self) -> Arc<EthereumProvider> {
        self.ethereum.clone()
    }

    /// Get the Polygon provider, if available
    pub fn polygon(&self) -> Option<Arc<EthereumProvider>> {
        self.polygon.clone()
    }

    /// Get the Arbitrum provider, if available
    pub fn arbitrum(&self) -> Option<Arc<EthereumProvider>> {
        self.arbitrum.clone()
    }

    /// Get the Optimism provider, if available
    pub fn optimism(&self) -> Option<Arc<EthereumProvider>> {
        self.optimism.clone()
    }

    /// Get a provider by chain ID
    pub fn by_chain_id(&self, chain_id: u64) -> Option<Arc<EthereumProvider>> {
        match chain_id {
            1 => Some(self.ethereum()),
            137 => self.polygon(),
            42161 => self.arbitrum(),
            10 => self.optimism(),
            _ => None,
        }
    }
}
