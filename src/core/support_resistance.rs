use crate::models::{LevelType, PriceLiquidity, SupportResistanceLevel, Token};
use crate::utils::calculate_support_resistance_strength;

/// Identify support and resistance levels from price liquidity data
pub fn identify_support_resistance(
    price_levels: &[PriceLiquidity],
    token0: &Token,
    token1: &Token,
    min_strength: f64,
) -> Vec<SupportResistanceLevel> {
    let mut levels = Vec::new();

    if price_levels.is_empty() {
        return levels;
    }

    // Calculate total liquidity for strength calculation
    let total_liquidity: f64 = price_levels
        .iter()
        .map(|pl| pl.token0_liquidity + pl.token1_liquidity)
        .sum();

    // Calculate price ranges
    let prices: Vec<f64> = price_levels.iter().map(|pl| pl.price).collect();
    let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let price_range = max_price - min_price;

    for (i, pl) in price_levels.iter().enumerate() {
        // Skip first and last price levels
        if i == 0 || i == price_levels.len() - 1 {
            continue;
        }

        let prev_pl = &price_levels[i - 1];
        let next_pl = &price_levels[i + 1];

        // Calculate combined liquidity at this level
        let combined_liquidity = pl.token0_liquidity + pl.token1_liquidity;

        // Calculate strength based on relative liquidity concentration
        let strength = calculate_support_resistance_strength(
            pl.price,
            combined_liquidity,
            total_liquidity,
            price_range,
        );

        // If strength above threshold, determine if it's support or resistance
        if strength >= min_strength {
            // Support: more buy-side (token0) liquidity
            // Resistance: more sell-side (token1) liquidity
            let level_type = if pl.token0_liquidity > pl.token1_liquidity {
                LevelType::Support
            } else if pl.token1_liquidity > pl.token0_liquidity {
                LevelType::Resistance
            } else {
                LevelType::Neutral
            };

            levels.push(SupportResistanceLevel {
                price: pl.price,
                strength,
                level_type,
                token0: token0.clone(),
                token1: token1.clone(),
            });
        }
    }

    // Sort by strength (descending)
    levels.sort_by(|a, b| {
        b.strength
            .partial_cmp(&a.strength)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    levels
}
