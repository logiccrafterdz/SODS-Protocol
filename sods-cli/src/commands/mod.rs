//! CLI command implementations.

pub mod agent;
pub mod chains;
pub mod daemon;
pub mod discover;
pub mod export_proof;
pub mod hash_pattern;
pub mod listen;
pub mod monitor;
pub mod register_agent;
pub mod registry;
pub mod symbols;
pub mod threats;
pub mod trend;
pub mod verify;
#[cfg(feature = "zk")]
pub mod zk_prove;
