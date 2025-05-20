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
}

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::DatabaseError(err.to_string())
    }
}
