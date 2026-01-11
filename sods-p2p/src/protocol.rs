//! Protocol types for P2P proof exchange.

use serde::{Deserialize, Serialize};

/// Protocol name for SODS proof exchange.
pub const PROTOCOL_NAME: &str = "/sods/proof/1.0.0";

/// Request for a behavioral proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    /// The symbol to verify (e.g., "Tf", "Dep")
    pub symbol: String,
    /// The block number to query
    pub block_number: u64,
}

/// Response containing a behavioral proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResponse {
    /// Serialized sods_core::Proof bytes
    pub proof_bytes: Vec<u8>,
    /// Behavioral Merkle Root for the block
    pub bmt_root: [u8; 32],
    /// Whether the request was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of symbol occurrences in block
    pub occurrences: usize,
}

impl ProofResponse {
    /// Create a successful response.
    pub fn success(proof_bytes: Vec<u8>, bmt_root: [u8; 32], occurrences: usize) -> Self {
        Self {
            proof_bytes,
            bmt_root,
            success: true,
            error: None,
            occurrences,
        }
    }

    /// Create an error response.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            proof_bytes: Vec::new(),
            bmt_root: [0u8; 32],
            success: false,
            error: Some(message.into()),
            occurrences: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = ProofRequest {
            symbol: "Dep".to_string(),
            block_number: 10002322,
        };
        
        // Test serde works
        let json = serde_json::to_string(&req).unwrap();
        let decoded: ProofRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(decoded.symbol, "Dep");
        assert_eq!(decoded.block_number, 10002322);
    }

    #[test]
    fn test_response_success() {
        let resp = ProofResponse::success(vec![1, 2, 3], [0xAB; 32], 5);
        assert!(resp.success);
        assert!(resp.error.is_none());
        assert_eq!(resp.occurrences, 5);
    }

    #[test]
    fn test_response_error() {
        let resp = ProofResponse::error("Symbol not found");
        assert!(!resp.success);
        assert!(resp.error.is_some());
    }
}
