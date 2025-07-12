use tel_core::config::Config;
use tel_core::error::Error;
use tel_core::models::{LiquidityDistribution, LiquidityWallsResponse, LiquidityWall, Side, Token};
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
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, warn, debug, error};
use std::collections::HashMap;

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
    Path((token0_addr, token1_addr)): Path<(String, String)>,
    Query(params): Query<LiquidityWallsQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<LiquidityWallsResponse>, ApiError> {
    // Validate addresses
    let token0_address = Address::parse_checksummed(&token0_addr, None).map_err(|e| ApiError {
        message: format!("Invalid token0 address format: {}", e),
        code: 400,
    })?;
    let token1_address = Address::parse_checksummed(&token1_addr, None).map_err(|e| ApiError {
        message: format!("Invalid token1 address format: {}", e),
        code: 400,
    })?;

    let chain_id = params.chain_id.unwrap_or(1);

    // Get tokens from database
    let token0 = state
        .storage
        .get_token(token0_address, chain_id)?
        .ok_or_else(|| ApiError {
            message: format!("Token {} not found in database", token0_address),
            code: 404,
        })?;
    let token1 = state
        .storage
        .get_token(token1_address, chain_id)?
        .ok_or_else(|| ApiError {
            message: format!("Token {} not found in database", token1_address),
            code: 404,
        })?;

    // Get liquidity distributions from database
    let dex_filter = params.dex.as_deref();
    let mut all_distributions: Vec<LiquidityDistribution> = Vec::new();
    
    // Define supported DEXes
    let dexes = if let Some(dex) = dex_filter {
        vec![dex.to_string()]
    } else {
        vec![
            "uniswap_v3".to_string(),
            "uniswap_v2".to_string(),
            "sushiswap".to_string(),
            "curve".to_string(),
            "balancer".to_string(),
        ]
    };

    // TODO: Collect and merge liquidity distributions from all relevant DEXes
    for dex in dexes {
        match state.storage.get_liquidity_distribution(
            token0_address,
            token1_address,
            &dex,
            chain_id,
        ) {
            Ok(Some(distribution)) => {
                info!("Found liquidity distribution for {} DEX", dex);
                all_distributions.push(distribution);
            }
            Ok(None) => {
                info!("No liquidity distribution found for {} DEX", dex);
            }
            Err(e) => {
                error!("Error getting liquidity distribution for {}: {}", dex, e);
            }
        }
    }

    if all_distributions.is_empty() {
        return Err(ApiError {
            message: "No liquidity distributions found".to_string(),
            code: 404,
        });
    }

    debug!("distributions: {:#?}", all_distributions);

    let distribution = all_distributions.first().unwrap();

    let current_price = distribution.current_price;

    let buy_walls = distribution
        .price_levels
        .iter()
        .filter(|d| d.side == Side::Buy)
        .map(|d| LiquidityWall {
            price_lower: d.lower_price,
            price_upper: d.upper_price,
            liquidity_value: d.token1_liquidity,
            dex_sources: HashMap::new(),
        })
        .collect();
    let sell_walls = distribution
        .price_levels
        .iter()
        .filter(|d| d.side == Side::Sell)
        .map(|d| LiquidityWall {
            price_lower: d.lower_price,
            price_upper: d.upper_price,
            liquidity_value: d.token0_liquidity * (d.upper_price + d.lower_price) / 2.0, // displayed in token1 value
            dex_sources: HashMap::new(),
        })
        .collect();

    let response = LiquidityWallsResponse {
        token0,
        token1,
        price: current_price,
        buy_walls,
        sell_walls,
        timestamp: chrono::Utc::now(),
    };

    Ok(Json(response))
}

/// Get token information
async fn get_token_info(
    Path((chain_id, address_str)): Path<(u64, String)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Token>, ApiError> {
    let address = Address::parse_checksummed(&address_str, None).map_err(|e| ApiError {
        message: format!("Invalid address format: {}", e),
        code: 400,
    })?;

    let token = state
        .storage
        .get_token(address, chain_id)?
        .ok_or_else(|| ApiError {
            message: format!("Token {} not found in database", address),
            code: 404,
        })?;
    Ok(Json(token))
}

/// Get pools by DEX and chain ID
async fn get_pools_by_dex(
    Path((dex, chain_id)): Path<(String, u64)>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<String>>, ApiError> {
    match state.storage.get_pools_by_dex(&dex, chain_id) {
        Ok(pools) => {
            let pool_addresses: Vec<String> =
                pools.iter().map(|pool| pool.address.to_string()).collect();
            Ok(Json(pool_addresses))
        }
        Err(e) => {
            warn!("Error getting pools by DEX: {}", e);
            // Return empty list instead of error for better UX
            Ok(Json(Vec::new()))
        }
    }
}

/// Run the API server
pub async fn run_server(config: Config) -> Result<(), Error> {
    // Initialize the database connection
    let storage = Arc::new(SqliteStorage::new(&config.database.url)?);

    // Initialize the provider manager
    let provider_manager = Arc::new(ProviderManager::new(&config.ethereum, None, None, None)?);

    let state = Arc::new(AppState {
        storage,
        config: config.clone(),
        provider_manager,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);
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
