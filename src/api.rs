use crate::config::Config;
use crate::error::Error;
use crate::models::{LiquidityWall, LiquidityWallsResponse, Token};
use crate::storage::Storage;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

/// Query parameters for the liquidity walls endpoint
#[derive(Debug, Deserialize)]
pub struct LiquidityWallsQuery {
    dex: Option<String>,
    chain_id: Option<u64>,
}

/// Application state shared across handlers
pub struct AppState {
    storage: Arc<dyn Storage>,
    config: Config,
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
        .route("/v1/tokens/:chain_id/:address", get(get_token))
        .route("/v1/pools/:dex/:chain_id", get(get_pools_by_dex))
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

/// Get liquidity walls for a token pair
async fn get_liquidity_walls(
    Path((token0, token1)): Path<(String, String)>,
    Query(params): Query<LiquidityWallsQuery>,
    State(state): State<Arc<AppState>>,
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
async fn get_token(
    Path((chain_id, address)): Path<(u64, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Token>, ApiError> {
    // This is a stub implementation
    let token = Token {
        address: Default::default(),
        symbol: "TOKEN".to_string(),
        name: "Test Token".to_string(),
        decimals: 18,
        chain_id,
    };

    Ok(Json(token))
}

/// Get pools by DEX and chain ID
async fn get_pools_by_dex(
    Path((dex, chain_id)): Path<(String, u64)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, ApiError> {
    // This is a stub implementation
    let pools = vec!["0x1234...".to_string(), "0x5678...".to_string()];

    Ok(Json(pools))
}

/// Run the API server
pub async fn run_server(config: Config) -> Result<(), Error> {
    // In a real implementation, we would initialize the database connection here
    let storage = Arc::new(crate::storage::SqliteStorage::new(&config.database.url)?);

    let state = Arc::new(AppState {
        storage,
        config: config.clone(),
    });

    let cors = CorsLayer::new().allow_origin(Any);

    let app = routes(state).layer(cors);

    let addr = format!("{}:{}", config.api.host, config.api.port)
        .parse::<SocketAddr>()
        .map_err(|e| Error::ApiError(format!("Invalid address: {}", e)))?;

    info!("Starting API server on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .map_err(|e| Error::ApiError(format!("Server error: {}", e)))?;

    Ok(())
}
