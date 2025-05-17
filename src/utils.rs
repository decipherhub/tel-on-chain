use crate::error::Error;
use crate::Address;
use std::str::FromStr;

/// Convert a string to an address
pub fn parse_address(address_str: &str) -> Result<Address, Error> {
    Address::from_str(address_str).map_err(|_| Error::InvalidAddress(address_str.to_string()))
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
