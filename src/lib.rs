pub mod api;
pub mod config;
// pub mod core; // Commented until compatible with Rust 1.81
// pub mod dexes; // Commented until compatible with Rust 1.81
pub mod error;
pub mod indexer;
pub mod models;
// pub mod providers; // Commented until compatible with Rust 1.81
pub mod storage;
pub mod utils;

pub use crate::error::Error;

// Temporary type alias for Address since alloy-primitives is removed
pub type Address = String;
