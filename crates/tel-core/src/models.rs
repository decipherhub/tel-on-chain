use alloy_primitives::Address;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a token in a DEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub address: Address,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub chain_id: u64,
}

/// Represents a liquidity pool in a DEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub address: Address,
    pub dex: String,
    pub chain_id: u64,
    pub tokens: Vec<Token>,
    pub creation_block: u64,
    pub creation_timestamp: DateTime<Utc>,
    pub last_updated_block: u64,
    pub last_updated_timestamp: DateTime<Utc>,
    /// Pool fee in 0.0001% units (e.g., 0.3% = 3000)
    pub fee: u32,
}

/// Represents a tick in Uniswap v3 or similar concentrated liquidity DEXs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityTick {
    pub pool_address: Address,
    pub tick_idx: i32,
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub price0: f64,
    pub price1: f64,
    pub block_number: u64,
    pub timestamp: DateTime<Utc>,
}

/// Represents aggregated liquidity at a specific price level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLiquidity {
    pub price: f64,
    pub token0_liquidity: f64,
    pub token1_liquidity: f64,
    pub timestamp: DateTime<Utc>,
}

/// Represents a distribution of liquidity across price ranges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityDistribution {
    pub token0: Token,
    pub token1: Token,
    pub dex: String,
    pub chain_id: u64,
    pub price_levels: Vec<PriceLiquidity>,
    pub timestamp: DateTime<Utc>,
}

/// Represents detected support/resistance levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportResistanceLevel {
    pub price: f64,
    pub strength: f64,
    pub level_type: LevelType,
    pub token0: Token,
    pub token1: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LevelType {
    Support,
    Resistance,
    Neutral,
}

/// Represents a swap impact calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapImpact {
    pub pool_address: Address,
    pub dex: String,
    pub token_in: Token,
    pub token_out: Token,
    pub amount_in: f64,
    pub amount_out: f64,
    pub price_impact_percent: f64,
    pub timestamp: DateTime<Utc>,
}

/// Represents a liquidity provider's position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPosition {
    pub provider_address: Address,
    pub pool_address: Address,
    pub dex: String,
    pub tokens: Vec<Token>,
    pub share_percent: f64,
    pub value_usd: Option<f64>,
    pub timestamp: DateTime<Utc>,
}

/// API response format for liquidity walls data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityWallsResponse {
    pub token0: Token,
    pub token1: Token,
    pub price: f64,
    pub buy_walls: Vec<LiquidityWall>,
    pub sell_walls: Vec<LiquidityWall>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityWall {
    pub price_lower: f64,
    pub price_upper: f64,
    pub liquidity_value: f64,
    pub dex_sources: HashMap<String, f64>,
}
