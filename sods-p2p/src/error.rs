//! Error types for SODS P2P operations.

use thiserror::Error;

/// Errors that can occur during P2P operations.
#[derive(Debug, Error)]
pub enum SodsP2pError {
    /// Network transport error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Protocol encoding/decoding error.
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// No peers available for query.
    #[error("No available peers for verification")]
    NoAvailablePeers,

    /// Social consensus failed.
    #[error("Consensus failure: only {agreeing}/{total} peers agreed")]
    ConsensusFailure {
        /// Number of agreeing peers
        agreeing: usize,
        /// Total peers queried
        total: usize,
    },

    /// Proof verification failed.
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Request timed out.
    #[error("Request timed out")]
    Timeout,

    /// Error from sods-verifier.
    #[error("Verifier error: {0}")]
    Verifier(#[from] sods_verifier::SodsVerifierError),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type alias for P2P operations.
pub type Result<T> = std::result::Result<T, SodsP2pError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = SodsP2pError::ConsensusFailure {
            agreeing: 1,
            total: 3,
        };
        assert!(err.to_string().contains("1/3"));
    }

    #[test]
    fn test_no_peers_error() {
        let err = SodsP2pError::NoAvailablePeers;
        assert!(err.to_string().contains("No available peers"));
    }
}
