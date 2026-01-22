//! Error types for the SODS Verifier.
//!
//! This module defines all error types that can occur during
//! RPC operations, symbol verification, and proof generation.

use thiserror::Error;

/// Errors that can occur during verification operations.
#[derive(Debug, Error)]
pub enum SodsVerifierError {
    /// RPC transport or network error.
    #[error("RPC error: {0}")]
    RpcError(String),

    /// RPC request timed out.
    #[error("RPC timeout after {attempts} attempts")]
    RpcTimeout {
        /// Number of retry attempts made
        attempts: u32,
    },

    /// The requested symbol was not found in the block.
    #[error("Symbol '{symbol}' not found in block {block_number}")]
    SymbolNotFound {
        /// The symbol that was searched for
        symbol: String,
        /// The block number that was searched
        block_number: u64,
    },

    /// The symbol is not in the supported registry.
    #[error("Unsupported symbol: '{0}'. Valid symbols: Tf, Dep, Wdw, Sw, LP+, LP-")]
    UnsupportedSymbol(String),

    /// Block number is out of valid range.
    #[error("Block {0} is out of range or does not exist")]
    BlockOutOfRange(u64),

    /// Block has no logs (empty block).
    #[error("Block {0} has no logs")]
    EmptyBlock(u64),

    /// Error from sods-core library.
    #[error("Core error: {0}")]
    Core(#[from] sods_core::SodsError),

    /// Receipt trie root does not match block header.
    #[error("Receipt proof invalid: computed root {computed} != header root {expected}")]
    InvalidReceiptProof {
        /// The computed receipts root from local trie
        computed: String,
        /// The expected receipts root from block header
        expected: String,
    },

    /// Block header logsBloom doesn't contain expected topic.
    #[error("LogsBloom filter indicates no relevant logs in block")]
    BloomFilterMismatch,

    /// RPC doesn't support header-anchored verification.
    #[error("RPC does not support receipt data required for trustless verification. Use --no-header-proof or switch providers")]
    ProofUnsupported,

    /// Failed to fetch block header.
    #[error("Failed to fetch block header for block {0}")]
    HeaderFetchFailed(u64),

    /// Failed to fetch receipts.
    #[error("Failed to fetch receipts for block {0}")]
    ReceiptFetchFailed(u64),
}

/// Result type alias for verifier operations.
pub type Result<T> = std::result::Result<T, SodsVerifierError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SodsVerifierError::SymbolNotFound {
            symbol: "Dep".to_string(),
            block_number: 12345,
        };
        assert!(err.to_string().contains("Dep"));
        assert!(err.to_string().contains("12345"));
    }

    #[test]
    fn test_unsupported_symbol_error() {
        let err = SodsVerifierError::UnsupportedSymbol("BadSymbol".to_string());
        assert!(err.to_string().contains("BadSymbol"));
        assert!(err.to_string().contains("Valid symbols"));
    }

    #[test]
    fn test_rpc_timeout_error() {
        let err = SodsVerifierError::RpcTimeout { attempts: 3 };
        assert!(err.to_string().contains("3 attempts"));
    }
}
