use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};

/// A behavioral commitment binds a BMT root to its block data.
/// 
/// This structure is signed off-chain and verified on-chain to ensure
/// that the BMT root used for verification is authentic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralCommitment {
    pub chain_id: u64,
    pub block_number: u64,
    pub receipts_root: [u8; 32],
    pub bmt_root: [u8; 32],
}

impl BehavioralCommitment {
    /// Create a new commitment.
    pub fn new(chain_id: u64, block_number: u64, receipts_root: [u8; 32], bmt_root: [u8; 32]) -> Self {
        Self {
            chain_id,
            block_number,
            receipts_root,
            bmt_root,
        }
    }

    /// Encode the commitment to bytes for signing (matching Solidity abi.encodePacked).
    /// 
    /// Format: (uint64, uint64, bytes32, bytes32)
    pub fn to_signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + 8 + 32 + 32);
        bytes.extend_from_slice(&self.chain_id.to_be_bytes());
        bytes.extend_from_slice(&self.block_number.to_be_bytes());
        bytes.extend_from_slice(&self.receipts_root);
        bytes.extend_from_slice(&self.bmt_root);
        bytes
    }

    /// Compute the Keccak256 hash of the commitment.
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Keccak::v256();
        hasher.update(&self.to_signing_bytes());
        let mut output = [0u8; 32];
        hasher.finalize(&mut output);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_signing_bytes() {
        let commitment = BehavioralCommitment::new(
            1,
            100,
            [0x11; 32],
            [0x22; 32],
        );

        let bytes = commitment.to_signing_bytes();
        assert_eq!(bytes.len(), 8 + 8 + 32 + 32);
        
        // uint64(1)
        assert_eq!(bytes[7], 1);
        // uint64(100)
        assert_eq!(bytes[15], 100);
        // receipts_root
        assert_eq!(bytes[16..48], [0x11; 32]);
        // bmt_root
        assert_eq!(bytes[48..80], [0x22; 32]);
    }

    #[test]
    fn test_commitment_hash() {
        let commitment = BehavioralCommitment::new(1, 1, [0; 32], [0; 32]);
        let hash = commitment.hash();
        assert_ne!(hash, [0; 32]);
    }
}
