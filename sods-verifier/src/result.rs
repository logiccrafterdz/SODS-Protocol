//! Verification result structure.
//!
//! Contains all information about a verification attempt including
//! timing metrics, proof data, and error information.

use serde::{Deserialize, Serialize};

use crate::header_anchor::VerificationMode;
use std::time::Duration;

/// Result of a symbol verification attempt.
///
/// Contains comprehensive information about the verification including
/// timing metrics for performance analysis.
///
/// # Example
///
/// ```rust
/// use sods_verifier::VerificationResult;
/// use sods_verifier::header_anchor::VerificationMode;
/// use std::time::Duration;
///
/// let result = VerificationResult {
///     symbol: "Dep".to_string(),
///     block_number: 10002322,
///     is_verified: true,
///     proof_size_bytes: 202,
///     merkle_root: Some(vec![0u8; 32]),
///     occurrences: 2,
///     confidence_score: 0.95,
///     verification_mode: VerificationMode::Trustless,
///     verification_time: Duration::from_micros(500),
///     rpc_fetch_time: Duration::from_millis(150),
///     total_time: Duration::from_millis(200),
///     error: None,
/// };
///
/// println!("Verified: {}", result.is_verified);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// The symbol that was queried.
    pub symbol: String,

    /// The block number that was searched.
    pub block_number: u64,

    /// Whether the symbol was found and verified.
    pub is_verified: bool,

    /// Size of the Merkle proof in bytes (0 if not found).
    pub proof_size_bytes: usize,

    /// The Behavioral Merkle Root (BMR) for the block.
    /// None if verification failed before BMT construction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merkle_root: Option<Vec<u8>>,

    /// Number of occurrences of the symbol in the block.
    pub occurrences: usize,

    /// Time spent on proof verification (excluding RPC).
    #[serde(with = "duration_millis")]
    pub verification_time: Duration,

    /// Time spent fetching data from RPC.
    #[serde(with = "duration_millis")]
    pub rpc_fetch_time: Duration,

    /// Total time from start to finish.
    #[serde(with = "duration_millis")]
    pub total_time: Duration,

    /// Confidence score (0.0 - 1.0) indicating reliability of the detection.
    pub confidence_score: f32,

    /// Verification mode indicating trust level.
    pub verification_mode: VerificationMode,

    /// Error message if verification failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl VerificationResult {
    /// Create a new result indicating successful verification.
    #[allow(clippy::too_many_arguments)]
    pub fn success(
        symbol: String,
        block_number: u64,
        proof_size_bytes: usize,
        merkle_root: [u8; 32],
        occurrences: usize,
        confidence_score: f32,
        verification_mode: VerificationMode,
        verification_time: Duration,
        rpc_fetch_time: Duration,
        total_time: Duration,
    ) -> Self {
        Self {
            symbol,
            block_number,
            is_verified: true,
            proof_size_bytes,
            merkle_root: Some(merkle_root.to_vec()),
            occurrences,
            confidence_score,
            verification_mode,
            verification_time,
            rpc_fetch_time,
            total_time,
            error: None,
        }
    }

    /// Create a new result indicating symbol not found.
    pub fn not_found(
        symbol: String,
        block_number: u64,
        merkle_root: Option<[u8; 32]>,
        verification_mode: VerificationMode,
        rpc_fetch_time: Duration,
        total_time: Duration,
    ) -> Self {
        Self {
            symbol: symbol.clone(),
            block_number,
            is_verified: false,
            proof_size_bytes: 0,
            merkle_root: merkle_root.map(|r| r.to_vec()),
            occurrences: 0,
            confidence_score: 0.0,
            verification_mode,
            verification_time: Duration::ZERO,
            rpc_fetch_time,
            total_time,
            error: Some(format!("Symbol '{}' not found in block", symbol)),
        }
    }

    /// Create a new result indicating an error.
    pub fn error(
        symbol: String,
        block_number: u64,
        error: String,
        verification_mode: VerificationMode,
        rpc_fetch_time: Duration,
        total_time: Duration,
    ) -> Self {
        Self {
            symbol,
            block_number,
            is_verified: false,
            proof_size_bytes: 0,
            merkle_root: None,
            occurrences: 0,
            confidence_score: 0.0,
            verification_mode,
            verification_time: Duration::ZERO,
            rpc_fetch_time,
            total_time,
            error: Some(error),
        }
    }
}

/// Custom serialization for Duration as milliseconds.
mod duration_millis {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

impl std::fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_verified {
            write!(
                f,
                "Symbol '{}' VERIFIED in block {} (score: {:.2}, {} occurrences, {} bytes proof, {:?} total)",
                self.symbol, self.block_number, self.confidence_score, self.occurrences, self.proof_size_bytes, self.total_time
            )
        } else {
            write!(
                f,
                "Symbol '{}' NOT FOUND in block {} ({})",
                self.symbol,
                self.block_number,
                self.error.as_deref().unwrap_or("unknown error")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_result() {
        let result = VerificationResult::success(
            "Dep".to_string(),
            12345,
            202,
            [0u8; 32],
            2,
            0.85,
            VerificationMode::Trustless,
            Duration::from_micros(500),
            Duration::from_millis(100),
            Duration::from_millis(150),
        );

        assert!(result.is_verified);
        assert_eq!(result.occurrences, 2);
        assert_eq!(result.confidence_score, 0.85);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_not_found_result() {
        let result = VerificationResult::not_found(
            "Wdw".to_string(),
            12345,
            Some([0u8; 32]),
            VerificationMode::Trustless,
            Duration::from_millis(100),
            Duration::from_millis(150),
        );

        assert!(!result.is_verified);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_display() {
        let result = VerificationResult::success(
            "Tf".to_string(),
            12345,
            202,
            [0u8; 32],
            5,
            0.9,
            VerificationMode::Trustless,
            Duration::from_micros(500),
            Duration::from_millis(100),
            Duration::from_millis(150),
        );

        let display = result.to_string();
        assert!(display.contains("VERIFIED"));
        assert!(display.contains("Tf"));
        assert!(display.contains("score: 0.90"));
    }
}
