use alloy_primitives::Address;
use crate::{Error, Result};
use std::str::FromStr;
use crate::models::{LiquidityDistribution, PriceLiquidity, Token};
use chrono::Utc;

/// Parse a string into an Address.
/// The string should be a valid hex string with or without the "0x" prefix.
pub fn parse_address(address_str: &str) -> Result<Address> {
    let address_str = address_str.trim_start_matches("0x");
    Address::from_str(address_str).map_err(|e| Error::InvalidAddress(e.to_string()))
}

/// Calculate price impact for constant product AMM
pub fn calculate_price_impact(reserve_in: f64, reserve_out: f64, amount_in: f64) -> f64 {
    // Price before swap
    let price_before = reserve_out / reserve_in;

    // Amount out using x * y = k formula
    let amount_out = (reserve_out * amount_in) / (reserve_in + amount_in);

    // New reserves after swap
    let new_reserve_in = reserve_in + amount_in;
    let new_reserve_out = reserve_out - amount_out;

    // Price after swap
    let price_after = new_reserve_out / new_reserve_in;

    // Price impact percentage
    ((price_before - price_after) / price_before) * 100.0
}

/// Calculate support/resistance strength from liquidity concentration
pub fn calculate_support_resistance_strength(
    _price_level: f64,
    liquidity: f64,
    total_liquidity: f64,
    price_range: f64,
) -> f64 {
    // Simple calculation: (liquidity / total_liquidity) * (1.0 / price_range)
    // Higher values indicate stronger support/resistance
    (liquidity / total_liquidity) * (1.0 / price_range) * 100.0
}

/// Format large numbers with K, M, B, T suffixes
pub fn format_large_number(num: f64) -> String {
    if num >= 1_000_000_000_000.0 {
        format!("{:.2}T", num / 1_000_000_000_000.0)
    } else if num >= 1_000_000_000.0 {
        format!("{:.2}B", num / 1_000_000_000.0)
    } else if num >= 1_000_000.0 {
        format!("{:.2}M", num / 1_000_000.0)
    } else if num >= 1_000.0 {
        format!("{:.2}K", num / 1_000.0)
    } else {
        format!("{:.2}", num)
    }
}

/// Merges two LiquidityDistribution objects into a single one.
///
/// This function assumes that both distributions are for the same token pair and chain.
/// It returns `None` if the distributions are inconsistent.
///
/// # Arguments
///
/// * `dist1` - The first LiquidityDistribution object.
/// * `dist2` - The second LiquidityDistribution object.
///
/// # Returns
///
/// * `Some(LiquidityDistribution)` if merging is successful.
/// * `None` if the distributions are inconsistent.
pub fn merge_two_liquidity_distributions(
    dist1: &LiquidityDistribution,
    dist2: &LiquidityDistribution,
) -> Option<LiquidityDistribution> {
    // 1. Validate that both distributions are for the same pair and chain
    if dist1.token0.address != dist2.token0.address
        || dist1.token1.address != dist2.token1.address
        || dist1.chain_id != dist2.chain_id
    {
        return None;
    }

    // 2. Merge the price_levels from both distributions
    let mut all_price_levels = dist1.price_levels.clone();
    all_price_levels.extend(dist2.price_levels.clone());

    // 3. Calculate the weighted average current_price
    let total_liquidity1: f64 = dist1.price_levels.iter().map(|p| p.token0_liquidity + p.token1_liquidity).sum();
    let total_liquidity2: f64 = dist2.price_levels.iter().map(|p| p.token0_liquidity + p.token1_liquidity).sum();
    let total_liquidity = total_liquidity1 + total_liquidity2;

    let merged_current_price = if total_liquidity > 0.0 {
        (dist1.current_price * total_liquidity1 + dist2.current_price * total_liquidity2) / total_liquidity
    } else {
        (dist1.current_price + dist2.current_price) / 2.0
    };

    // 4. Create the merged LiquidityDistribution
    Some(LiquidityDistribution {
        token0: dist1.token0.clone(),
        token1: dist1.token1.clone(),
        current_price: merged_current_price,
        dex: "aggregated".to_string(), // Mark as aggregated data
        chain_id: dist1.chain_id,
        price_levels: all_price_levels,
        timestamp: Utc::now(), // Set new timestamp
    })
}

/// Buckets price levels into uniform intervals
pub fn bucket_price_levels(price_levels: Vec<PriceLiquidity>, current_price: f64, bucket_size: f64) -> Vec<PriceLiquidity> {
    use std::collections::HashMap;
    
    let mut buckets: HashMap<i32, PriceLiquidity> = HashMap::new();
    
    for level in price_levels {
        let mid_price = (level.lower_price + level.upper_price) / 2.0;
        let bucket_index = ((mid_price / current_price - 1.0) / bucket_size).round() as i32;
        
        let bucket_center = current_price * (1.0 + bucket_index as f64 * bucket_size);
        let bucket_lower = bucket_center * (1.0 - bucket_size / 2.0);
        let bucket_upper = bucket_center * (1.0 + bucket_size / 2.0);
        
        buckets.entry(bucket_index)
            .and_modify(|existing| {
                existing.token0_liquidity += level.token0_liquidity;
                existing.token1_liquidity += level.token1_liquidity;
            })
            .or_insert(PriceLiquidity {
                side: level.side,
                lower_price: bucket_lower,
                upper_price: bucket_upper,
                token0_liquidity: level.token0_liquidity,
                token1_liquidity: level.token1_liquidity,
                timestamp: level.timestamp,
            });
    }
    
    buckets.into_values().collect()
}


       