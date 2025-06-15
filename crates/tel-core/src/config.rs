use anyhow::Result;
use config::{Config as ConfigLib, File};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct RpcConfig {
    pub url: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexerConfig {
    pub interval_secs: u64,
    pub batch_size: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SupportedDex {
    pub name: String,
    pub chain_id: u64,
    pub factory_address: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub ethereum: RpcConfig,
    pub polygon: Option<RpcConfig>,
    pub arbitrum: Option<RpcConfig>,
    pub optimism: Option<RpcConfig>,
    pub database: DatabaseConfig,
    pub api: ApiConfig,
    pub indexer: IndexerConfig,
    pub dexes: Vec<SupportedDex>,
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let config = ConfigLib::builder()
        .add_source(File::from(path.as_ref()))
        .build()?;

    Ok(config.try_deserialize()?)
}

/// Creates a default config file if it doesn't exist
pub fn ensure_default_config() -> Result<()> {
    let config_dir = Path::new("config");
    if !config_dir.exists() {
        std::fs::create_dir_all(config_dir)?;
    }

    let default_config_path = config_dir.join("default.toml");
    if !default_config_path.exists() {
        let default_config = r#"
# tel-on-chain Default Configuration

[ethereum]
url = "https://eth.llamarpc.com"
timeout_secs = 30

[database]
url = "sqlite_tel_on_chain.db"

[api]
host = "127.0.0.1"
port = 8080

[indexer]
interval_secs = 600  # 10 minutes
batch_size = 1000

# Supported DEXes
[[dexes]]
name = "uniswap_v2"
chain_id = 1
factory_address = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f"
enabled = true

[[dexes]]
name = "uniswap_v3"
chain_id = 1
factory_address = "0x1F98431c8aD98523631AE4a59f267346ea31F984"
enabled = true

[[dexes]]
name = "sushiswap"
chain_id = 1
factory_address = "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"
enabled = true
"#;
        std::fs::write(default_config_path, default_config)?;
    }

    Ok(())
}
