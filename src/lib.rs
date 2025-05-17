pub mod api;
pub mod config;
pub mod core;
pub mod dexes;
pub mod error;
pub mod indexer;
pub mod models;
pub mod providers;
pub mod storage;
pub mod utils;

pub use crate::error::Error;

// Use the proper Address type from alloy-primitives instead of the String placeholder
pub use alloy_primitives::Address;
