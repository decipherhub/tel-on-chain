use crate::models::{LiquidityDistribution, LiquidityWall, PriceLiquidity};
use std::collections::HashMap;

/// Identifies liquidity walls from a set of liquidity distributions
pub fn identify_walls(
    distributions: &[LiquidityDistribution],
    price_ranges: &[(f64, f64)],
) -> (Vec<LiquidityWall>, Vec<LiquidityWall>) {
    let mut buy_walls = Vec::new();
    let mut sell_walls = Vec::new();

    // This is a simplified implementation
    // In a real system, we would analyze the distributions to find significant
    // concentrations of liquidity at specific price levels

    for &(price_lower, price_upper) in price_ranges {
        let buy_wall = LiquidityWall {
            price_lower,
            price_upper,
            liquidity_value: 100000.0, // Placeholder
            dex_sources: HashMap::new(),
        };

        let sell_wall = LiquidityWall {
            price_lower: price_upper,
            price_upper: price_upper * 1.05,
            liquidity_value: 100000.0, // Placeholder
            dex_sources: HashMap::new(),
        };

        buy_walls.push(buy_wall);
        sell_walls.push(sell_wall);
    }

    (buy_walls, sell_walls)
}

/// Groups price levels into ranges for simplified visualization
pub fn group_price_levels(
    price_levels: &[PriceLiquidity],
    num_groups: usize,
) -> Vec<PriceLiquidity> {
    if price_levels.is_empty() || num_groups == 0 {
        return Vec::new();
    }

    // Find min and max prices
    let min_price = price_levels
        .iter()
        .map(|p| p.price)
        .fold(f64::INFINITY, f64::min);
    let max_price = price_levels
        .iter()
        .map(|p| p.price)
        .fold(f64::NEG_INFINITY, f64::max);

    // Calculate the range size
    let range_size = (max_price - min_price) / num_groups as f64;

    // Group price levels
    let mut grouped = Vec::new();
    for i in 0..num_groups {
        let group_min = min_price + range_size * i as f64;
        let group_max = group_min + range_size;

        // Filter price levels in this range
        let levels_in_range: Vec<_> = price_levels
            .iter()
            .filter(|p| p.price >= group_min && p.price < group_max)
            .collect();

        if !levels_in_range.is_empty() {
            // Average price in the range
            let avg_price: f64 =
                levels_in_range.iter().map(|p| p.price).sum::<f64>() / levels_in_range.len() as f64;

            // Sum of liquidity in the range
            let token0_liquidity: f64 = levels_in_range.iter().map(|p| p.token0_liquidity).sum();
            let token1_liquidity: f64 = levels_in_range.iter().map(|p| p.token1_liquidity).sum();

            // Use the timestamp of the latest price level
            let timestamp = levels_in_range
                .iter()
                .max_by_key(|p| p.timestamp)
                .map(|p| p.timestamp)
                .unwrap_or_else(chrono::Utc::now);

            grouped.push(PriceLiquidity {
                price: avg_price,
                token0_liquidity,
                token1_liquidity,
                timestamp,
            });
        }
    }

    grouped
}
