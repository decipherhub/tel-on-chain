use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("DEX error: {0}")]
    DexError(String),

    #[error("Unknown error: {0}")]
    Unknown(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Unknown DEX: {0}")]
    UnknownDEX(String),

    #[error("Not implemented")]
    NotImplemented,

    #[error("Uniswap V3 SDK error: {0}")]
    UniswapV3Error(String),

    #[error("Uniswap SDK Core error: {0}")]
    UniswapCoreError(String),

    #[error("Conversion error: {0}")]
    ConversionError(String),
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::DatabaseError(err.to_string())
    }
}

impl From<uniswap_v3_sdk::error::Error> for Error {
    fn from(err: uniswap_v3_sdk::error::Error) -> Self {
        Error::UniswapV3Error(err.to_string())
    }
}

impl From<uniswap_sdk_core::error::Error> for Error {
    fn from(err: uniswap_sdk_core::error::Error) -> Self {
        Error::UniswapCoreError(err.to_string())
    }
}

impl From<std::num::TryFromIntError> for Error {
    fn from(err: std::num::TryFromIntError) -> Self {
        Error::ConversionError(err.to_string())
    }
}

// Generic implementation for any conversion error
impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::ConversionError(err.to_string())
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        Error::ConversionError("Infallible error".to_string())
    }
}
