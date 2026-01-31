//! Causal proof representation and verification.
//!
//! This module provides the `CausalProof` which represents a cryptographic proof
//! that a specific event exists in a `CausalMerkleTree`.

use ethers::types::H256;
use serde::{Deserialize, Serialize};
use tiny_keccak::{Hasher, Keccak};
use crate::event::CausalEvent;

/// A Merkle inclusion proof for a causal event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalProof {
    /// The event being proven.
    pub event: CausalEvent,
    /// The Merkle path (sibling hashes).
    pub merkle_path: Vec<H256>,
    /// Whether each sibling is a left sibling (true) or right sibling (false).
    pub is_left_path: Vec<bool>,
    /// The root hash of the tree.
    pub root: H256,
}

impl CausalProof {
    /// Verifies the inclusion proof.
    ///
    /// Reconstructs the root hash from the event and the Merkle path.
    /// Returns true if the reconstructed root matches `self.root`.
    pub fn verify(&self) -> bool {
        // Step 1: Compute leaf hash
        let serialized = self.event.rlp_encode();
        let mut current_hash = [0u8; 32];
        let mut hasher = Keccak::v256();
        hasher.update(&serialized);
        hasher.finalize(&mut current_hash);
        let mut current_hash = H256::from(current_hash);

        // Step 2: Traverse path up to root
        for (i, sibling_hash) in self.merkle_path.iter().enumerate() {
            let mut hasher = Keccak::v256();
            
            if self.is_left_path[i] {
                // Sibling is on the left
                hasher.update(sibling_hash.as_bytes());
                hasher.update(current_hash.as_bytes());
            } else {
                // Sibling is on the right
                hasher.update(current_hash.as_bytes());
                hasher.update(sibling_hash.as_bytes());
            }

            let mut next_hash = [0u8; 32];
            hasher.finalize(&mut next_hash);
            current_hash = H256::from(next_hash);
        }

        // Step 3: Compare with expected root
        current_hash == self.root
    }
}
