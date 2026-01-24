//! Error types for the SODS core library.
//!
//! This module defines all error types that can occur during
//! symbol parsing, tree construction, and proof verification.

use thiserror::Error;

/// Errors that can occur in SODS core operations.
#[derive(Debug, Error)]
pub enum SodsError {
    /// Symbol was not found in the tree at the specified position.
    #[error("Symbol not found in tree: '{symbol}' at log_index {log_index}")]
    SymbolNotFound {
        /// The symbol that was searched for
        symbol: String,
        /// The log index that was searched for
        log_index: u32,
    },

    /// Merkle proof verification failed.
    #[error("Proof verification failed: computed root does not match expected")]
    VerificationFailed,

    /// Invalid or malformed proof data.
    #[error("Invalid proof data: {0}")]
    InvalidProof(String),

    /// Serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Unknown event topic (not in symbol registry).
    #[error("Unknown event topic: {0}")]
    UnknownTopic(String),

    /// Invalid pattern sequence.
    #[error("Pattern error: {0}")]
    PatternError(String),

    /// Internal error (RLP parsing, MPT verification, etc.)
    #[error("Internal error: {0}")]
    InternalError(String),

    /// Configuration or persistence error.
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type alias for SODS operations.
pub type Result<T> = std::result::Result<T, SodsError>;
