//! Error types for the SODS causal event model.
//!
//! This module defines all error types that can occur during
//! causal event creation, validation, and recording.

use thiserror::Error;

/// Errors that can occur in causal event operations.
#[derive(Debug, Error)]
pub enum CausalError {
    /// Invalid Ethereum address provided.
    #[error("Invalid agent address: {0}")]
    InvalidAgentAddress(String),

    /// Sequence gap detected within a transaction.
    #[error("Event sequence gap detected: expected {expected}, got {actual}")]
    SequenceGap {
        /// Expected sequence index
        expected: u32,
        /// Actual sequence index received
        actual: u32,
    },

    /// Nonce gap detected across transactions.
    #[error("Nonce gap detected: expected {expected}, got {actual}")]
    NonceGap {
        /// Expected nonce value
        expected: u64,
        /// Actual nonce received
        actual: u64,
    },

    /// Invalid result value provided.
    #[error("Invalid result value: {0}. Must be 'success', 'failure', 'partial', or 'timeout'")]
    InvalidResult(String),

    /// Serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type alias for causal operations.
pub type Result<T> = std::result::Result<T, CausalError>;
