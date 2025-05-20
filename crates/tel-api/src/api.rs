use tel_core::config::Config;
use tel_core::dexes::utils::get_token;
use tel_core::error::Error;
use tel_core::models::{LiquidityWall, LiquidityWallsResponse, Token};
use tel_core::providers::ProviderManager;
use tel_core::storage::Storage;
use tel_core::storage::SqliteStorage;
use alloy_primitives::Address;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

/// Query parameters for liquidity walls endpoint
#[derive(Debug, Deserialize)]
pub struct LiquidityWallsQuery {
    dex: Option<String>,
    chain_id: Option<u64>,
}

/// Application state shared across all routes
pub struct AppState {
    storage: Arc<dyn Storage>,
    config: Config,
    provider_manager: Arc<ProviderManager>,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    message: String,
    code: u16,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let body = Json(self);
        (status, body).into_response()
    }
}

/// Convert Error to ApiError
impl From<Error> for ApiError {
    fn from(err: Error) -> Self {
        match err {
            Error::DexError(msg) => ApiError {
                message: msg,
                code: 400,
            },
            Error::InvalidAddress(msg) => ApiError {
                message: format!("Invalid address: {}", msg),
                code: 400,
            },
            Error::ProviderError(msg) => ApiError {
                message: format!("Provider error: {}", msg),
                code: 500,
            },
            Error::Unknown(msg) => ApiError {
                message: msg,
                code: 500,
            },
            _ => ApiError {
                message: format!("Internal server error: {}", err),
                code: 500,
            },
        }
    }
}

/// Setup the API routes
fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route(
            "/v1/liquidity/walls/:token0/:token1",
            get(get_liquidity_walls),
        )
        .route("/v1/tokens/:chain_id/:address", get(get_token_info))
        .route("/v1/pools/:dex/:chain_id", get(get_pools_by_dex))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

/// Get liquidity walls for a token pair
async fn get_liquidity_walls(
    Path((_token0, _token1)): Path<(String, String)>,
    Query(params): Query<LiquidityWallsQuery>,
    State(_state): State<Arc<AppState>>,
) -> Result<Json<LiquidityWallsResponse>, ApiError> {
    // This is a stub implementation
    // In a real implementation, we would:
    // 1. Look up token addresses
    // 2. Query the database for liquidity distributions
    // 3. Analyze and aggregate them into buy/sell walls
    // 4. Return the result

    let response = LiquidityWallsResponse {
        token0: Token {
            address: Default::default(),
            symbol: "TOKEN0".to_string(),
            name: "Token 0".to_string(),
            decimals: 18,
            chain_id: params.chain_id.unwrap_or(1),
        },
        token1: Token {
            address: Default::default(),
            symbol: "TOKEN1".to_string(),
            name: "Token 1".to_string(),
            decimals: 18,
            chain_id: params.chain_id.unwrap_or(1),
        },
        price: 1000.0,
        buy_walls: vec![LiquidityWall {
            price_lower: 950.0,
            price_upper: 990.0,
            liquidity_value: 100000.0,
            dex_sources: HashMap::new(),
        }],
        sell_walls: vec![LiquidityWall {
            price_lower: 1010.0,
            price_upper: 1050.0,
            liquidity_value: 200000.0,
            dex_sources: HashMap::new(),
        }],
        timestamp: chrono::Utc::now(),
    };

    Ok(Json(response))
}

/// Get token information
async fn get_token_info(
    Path((chain_id, address)): Path<(u64, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Token>, ApiError> {
    let provider = state
        .provider_manager
        .by_chain_id(chain_id)
        .ok_or_else(|| ApiError {
            message: format!("No provider found for chain {}", chain_id),
            code: 400,
        })?;
    let address = Address::parse_checksummed(&address, None).map_err(|e| ApiError {
        message: format!("Invalid address format: {}", e),
        code: 400,
    })?;

    let token = get_token(provider.clone(), address, chain_id).await?;
    Ok(Json(token))
}

/// Get pools by DEX and chain ID
async fn get_pools_by_dex(
    Path((_dex, _chain_id)): Path<(String, u64)>,
    State(_state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, ApiError> {
    // This is a stub implementation
    let pools = vec!["0x1234...".to_string(), "0x5678...".to_string()];

    Ok(Json(pools))
}

/// Run the API server
pub async fn run_server(config: Config) -> Result<(), Error> {
    // Initialize the database connection
    let storage = Arc::new(SqliteStorage::new(&config.database.url)?);

    // Initialize the provider manager
    let provider_manager = Arc::new(ProviderManager::new(
        &config.ethereum,
        config.polygon.as_ref(),
        config.arbitrum.as_ref(),
        config.optimism.as_ref(),
    )?);

    let state = Arc::new(AppState {
        storage,
        config: config.clone(),
        provider_manager,
    });

    let cors = CorsLayer::new().allow_origin(Any);
    let app = routes(state).layer(cors);

    // Use port 8081 instead of the configured port
    let addr = format!("{}:{}", config.api.host, 8081)
        .parse::<SocketAddr>()
        .map_err(|e| Error::Unknown(format!("Failed to parse socket address: {}", e)))?;

    info!("Starting API server on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .map_err(|e| Error::Unknown(format!("Server error: {}", e)))?;

    Ok(())
}
