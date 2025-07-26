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

/// Creates a synthetic liquidity distribution for an A-C pair from A-B and B-C pairs.
///
/// # Arguments
///
/// * `dist_ab` - Liquidity distribution for the A-B pair.
/// * `dist_bc` - Liquidity distribution for the B-C pair.
///
/// # Returns
///
/// * `Some(LiquidityDistribution)` if the synthetic distribution can be created.
/// * `None` if the pairs are not linkable (e.g., B does not match).
pub fn merge_synthetic_liquidity_distributions(
    dist_ab: &LiquidityDistribution,
    dist_bc: &LiquidityDistribution,
) -> Option<LiquidityDistribution> {
    // 1. Identify tokens A, B, and C
    let token_a: &Token;
    let token_b_from_ab: &Token;
    let token_b_from_bc: &Token;
    let token_c: &Token;

    if dist_ab.token1.address == dist_bc.token0.address {
        token_a = &dist_ab.token0;
        token_b_from_ab = &dist_ab.token1;
        token_b_from_bc = &dist_bc.token0;
        token_c = &dist_bc.token1;
    } else {
        // This simple example only handles A/B - B/C. More complex logic could handle A/B - C/B etc.
        return None;
    }

    // 2. Calculate synthetic current price
    let synthetic_price = dist_ab.current_price * dist_bc.current_price;

    // 3. Create synthetic price levels (simplified approach)
    let mut synthetic_levels = Vec::new();
    for level_ab in &dist_ab.price_levels {
        for level_bc in &dist_bc.price_levels {
            // The amount of A that can be swapped for B
            let a_liquidity = level_ab.token0_liquidity;
            // The amount of B available from the first swap
            let b_from_a = a_liquidity * level_ab.upper_price; // Simplified

            // The amount of B required for the second swap
            let b_needed_for_c = level_bc.token0_liquidity;

            // The bottleneck is the minimum of B available and B needed
            let bottleneck_b = b_from_a.min(b_needed_for_c);

            if bottleneck_b > 0.0 {
                let synthetic_a_liquidity = bottleneck_b / level_ab.upper_price;
                let synthetic_c_liquidity = bottleneck_b * level_bc.upper_price;

                synthetic_levels.push(PriceLiquidity {
                    side: level_ab.side, // This is a simplification
                    lower_price: level_ab.lower_price * level_bc.lower_price,
                    upper_price: level_ab.upper_price * level_bc.upper_price,
                    token0_liquidity: synthetic_a_liquidity, // Token A
                    token1_liquidity: synthetic_c_liquidity, // Token C
                    timestamp: Utc::now(),
                });
            }
        }
    }

    // In a real-world scenario, you would need to re-bucket/aggregate `synthetic_levels`
    // as the number of levels can become very large (N*M).

    Some(LiquidityDistribution {
        token0: token_a.clone(),
        token1: token_c.clone(),
        current_price: synthetic_price,
        dex: "synthetic".to_string(),
        chain_id: dist_ab.chain_id, // Assuming same chain
        price_levels: synthetic_levels,
        timestamp: Utc::now(),
    })
}
