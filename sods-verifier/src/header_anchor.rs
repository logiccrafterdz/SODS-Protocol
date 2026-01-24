//! Block Header Anchored Verification.
//!
//! This module provides cryptographic anchoring of logs to Ethereum block headers
//! using receipt trie validation. It ensures that logs returned by RPC are genuine
//! by verifying they belong to receipts that hash to the block's `receiptsRoot`.
//!
//! ## Trust Model
//!
//! - **Trustless Mode**: Logs verified against block header via receipt trie
//! - **RPC Only Mode**: Logs accepted from RPC without proof (legacy behavior)
//!
//! ## Process
//!
//! 1. Fetch block header with `receiptsRoot`
//! 2. Check `logsBloom` for expected event topics (fast path rejection)
//! 3. Fetch all transaction receipts
//! 4. RLP-encode receipts and build Patricia trie
//! 5. Compare computed root with `receiptsRoot`
//! 6. If match: logs are authentic. If mismatch: RPC is lying.

use ethers_core::types::{Bloom, Log, TransactionReceipt, H256};
use sha3::{Digest, Keccak256};

// use crate::error::{Result, SodsVerifierError};

/// Verification mode indicating the trust level of the verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationMode {
    /// Logs are cryptographically anchored to block header via receipt trie.
    /// This is trustless verification — the RPC cannot fabricate logs.
    Trustless,

    /// Logs are fetched individually and verified via Merkle-Patricia proofs.
    /// This eliminates reliance on eth_getLogs and bulk receipt fetching.
    ZeroRpc,

    /// Logs are accepted from RPC without cryptographic proof.
    /// This requires trusting the RPC provider.
    RpcOnly,
}

impl std::fmt::Display for VerificationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trustless => write!(f, "Trustless — Block Header Anchored"),
            Self::ZeroRpc => write!(f, "Zero-RPC — Storage Proof Verified"),
            Self::RpcOnly => write!(f, "RPC Only — Requires Trust in Provider"),
        }
    }
}

impl Default for VerificationMode {
    fn default() -> Self {
        Self::Trustless
    }
}

/// Minimal block header containing fields needed for validation.
#[derive(Debug, Clone)]
pub struct BlockHeader {
    /// Block number
    pub number: u64,
    /// Hash of the block
    pub hash: H256,
    /// Root of the receipt trie
    pub receipts_root: H256,
    /// Parent beacon block root (EIP-4788)
    pub parent_beacon_block_root: Option<H256>,
    /// Block timestamp
    pub timestamp: u64,
    /// Bloom filter for logs in this block
    pub logs_bloom: Bloom,
}

/// Receipt anchor validation result.
#[derive(Debug)]
pub struct AnchorValidation {
    /// Whether the validation passed
    pub is_valid: bool,
    /// Computed receipts root
    pub computed_root: H256,
    /// Expected receipts root from header
    pub expected_root: H256,
    /// Number of receipts validated
    pub receipt_count: usize,
}

/// Check if a bloom filter contains a specific topic.
///
/// Bloom filters in Ethereum are 2048-bit (256-byte) vectors using Keccak-256
/// with a specific bit-setting algorithm defined in the Yellow Paper.
///
/// # Arguments
/// * `bloom` - The 2048-bit logs bloom filter
/// * `topic` - The 32-byte event topic to check
///
/// # Returns
/// `true` if the topic _might_ be in the bloom (may have false positives)
/// `false` if the topic is definitely NOT in the bloom
pub fn bloom_contains_topic(bloom: &Bloom, topic: &H256) -> bool {
    // Ethereum bloom filter uses 3 hash functions
    // Each extracts 11 bits from keccak256(x) to determine bit positions
    let hash = Keccak256::digest(topic.as_bytes());
    
    for i in 0..3 {
        let bit_pair_index = i * 2;
        let high = hash[bit_pair_index] as usize;
        let low = hash[bit_pair_index + 1] as usize;
        let bit_index = ((high << 8) | low) & 0x7FF; // 11 bits = 0-2047
        
        let byte_index = 255 - (bit_index / 8); // bloom is big-endian
        let bit_offset = bit_index % 8;
        
        if bloom.0[byte_index] & (1 << bit_offset) == 0 {
            return false;
        }
    }
    
    true
}

/// Check if a bloom filter contains any of the given topics.
pub fn bloom_contains_any_topic(bloom: &Bloom, topics: &[H256]) -> bool {
    topics.iter().any(|t| bloom_contains_topic(bloom, t))
}

/// Compute the Merkle-Patricia trie root from a list of receipts.
///
/// The receipt trie is ordered by transaction index (0, 1, 2, ...).
/// Keys are RLP-encoded indices, values are RLP-encoded receipts.
pub fn compute_receipts_root(receipts: &[TransactionReceipt]) -> H256 {
    sods_core::header_anchor::compute_receipts_root(receipts)
}

/// Verify that receipts match the block header's receiptsRoot.
///
/// # Arguments
/// * `receipts` - Transaction receipts to verify (must be in tx index order)
/// * `header` - Block header containing the expected receiptsRoot
///
/// # Returns
/// `AnchorValidation` with the result of the check
pub fn verify_receipts_against_header(
    receipts: &[TransactionReceipt],
    header: &BlockHeader,
) -> AnchorValidation {
    let computed_root = compute_receipts_root(receipts);

    AnchorValidation {
        is_valid: computed_root == header.receipts_root,
        computed_root,
        expected_root: header.receipts_root,
        receipt_count: receipts.len(),
    }
}

/// Validate that a log exists within a receipt.
///
/// Checks that the log's address, topics, and data match a log in the receipt.
pub fn validate_log_in_receipt(log: &Log, receipt: &TransactionReceipt) -> bool {
    receipt.logs.iter().any(|r_log| {
        r_log.address == log.address
            && r_log.topics == log.topics
            && r_log.data == log.data
            && r_log.log_index == log.log_index
    })
}

/// Extract all logs from validated receipts.
pub fn extract_logs_from_receipts(receipts: &[TransactionReceipt]) -> Vec<Log> {
    receipts.iter().flat_map(|r| r.logs.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers_core::types::{Address, Bytes, U256};

    fn create_test_log() -> Log {
        Log {
            address: Address::zero(),
            topics: vec![H256::zero()],
            data: Bytes::from(vec![1, 2, 3]),
            block_hash: Some(H256::zero()),
            block_number: Some(ethers_core::types::U64::from(1)),
            transaction_hash: Some(H256::zero()),
            transaction_index: Some(ethers_core::types::U64::from(0)),
            log_index: Some(U256::from(0)),
            transaction_log_index: None,
            log_type: None,
            removed: Some(false),
        }
    }

    #[test]
    fn test_verification_mode_display() {
        assert_eq!(
            VerificationMode::Trustless.to_string(),
            "Trustless — Block Header Anchored"
        );
        assert_eq!(
            VerificationMode::RpcOnly.to_string(),
            "RPC Only — Requires Trust in Provider"
        );
    }

    #[test]
    fn test_verification_mode_default() {
        assert_eq!(VerificationMode::default(), VerificationMode::Trustless);
    }

    #[test]
    fn test_bloom_contains_topic() {
        // Empty bloom should not contain any topic
        let empty_bloom = Bloom::default();
        let topic = H256::random();
        
        // Manual bloom check should return false for empty bloom
        assert!(!bloom_contains_topic(&empty_bloom, &topic));

        // Create a bloom with a specific topic
        let mut bloom = Bloom::default();
        // Manually set bits for a comprehensive test logic
        // Let's use a known topic hash to verify bit setting
        let topic_hash = Keccak256::digest(topic.as_bytes());
        
        for i in 0..3 {
            let bit_pair_index = i * 2;
            let high = topic_hash[bit_pair_index] as usize;
            let low = topic_hash[bit_pair_index + 1] as usize;
            let bit_index = ((high << 8) | low) & 0x7FF;
            let byte_index = 255 - (bit_index / 8);
            let bit_offset = bit_index % 8;
            bloom.0[byte_index] |= 1 << bit_offset;
        }

        // Should now contain the topic
        assert!(bloom_contains_topic(&bloom, &topic));

        // Random check - extremely unlikely to match
        // Note: Bloom filters have false positives, but with 3 bits set out of 2048,
        // probability of collision for random topic is very low (~ 2.5e-9)
        let other = H256::random();
        // This assertion *could* theoretically fail but is statistically safe for unit test
        assert!(!bloom_contains_topic(&bloom, &other)); 
    }

    #[test]
    fn test_rlp_encode_index() {
        // Index 0 should encode to 0x80 (empty string in RLP)
        let encoded = rlp_encode_index(0);
        assert_eq!(encoded, vec![0x80]);

        // Index 127 should encode to single byte
        let encoded = rlp_encode_index(127);
        assert_eq!(encoded, vec![127]);

        // Index 128 should encode with length prefix
        let encoded = rlp_encode_index(128);
        assert_eq!(encoded, vec![0x81, 128]);
    }

    #[test]
    fn test_empty_receipts_root() {
        let receipts: Vec<TransactionReceipt> = vec![];
        let root = compute_receipts_root(&receipts);

        // Empty trie root is keccak256(0x80)
        let expected = H256::from_slice(&Keccak256::digest(&[0x80]));
        assert_eq!(root, expected);
    }

    #[test]
    fn test_validate_log_in_receipt() {
        let log = create_test_log();
        let receipt = TransactionReceipt {
            logs: vec![log.clone()],
            ..Default::default()
        };

        assert!(validate_log_in_receipt(&log, &receipt));

        // Different log should not match
        let mut other_log = log.clone();
        other_log.address = Address::random();
        assert!(!validate_log_in_receipt(&other_log, &receipt));
    }

    #[test]
    fn test_extract_logs_from_receipts() {
        let log1 = create_test_log();
        let mut log2 = create_test_log();
        log2.log_index = Some(U256::from(1));

        let receipt1 = TransactionReceipt {
            logs: vec![log1.clone()],
            ..Default::default()
        };
        let receipt2 = TransactionReceipt {
            logs: vec![log2.clone()],
            ..Default::default()
        };

        let logs = extract_logs_from_receipts(&[receipt1, receipt2]);
        assert_eq!(logs.len(), 2);
    }
}
