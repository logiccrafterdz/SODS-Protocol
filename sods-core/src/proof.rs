//! Merkle inclusion proof for behavioral symbols.
//!
//! This module provides the `Proof` struct which represents a cryptographic
//! proof that a specific symbol exists in a Behavioral Merkle Tree.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Result, SodsError};

/// A Merkle inclusion proof for a behavioral symbol.
///
/// Contains all the information needed to verify that a symbol exists
/// in a Behavioral Merkle Tree without having access to the full tree.
///
/// # Verification
///
/// To verify a proof, the verifier:
/// 1. Starts with the `leaf_hash`
/// 2. For each sibling in `path`:
///    - If `directions[i]` is true, compute `H(current || sibling)`
///    - If `directions[i]` is false, compute `H(sibling || current)`
/// 3. Compare final hash with expected root
///
/// # Serialization
///
/// Proofs can be serialized to compact binary format using `serialize()`
/// and deserialized using `deserialize()`.
///
/// # Example
///
/// ```rust
/// use sods_core::{BehavioralMerkleTree, BehavioralSymbol};
///
/// let symbols = vec![
///     BehavioralSymbol::new("Tf", 0),
///     BehavioralSymbol::new("Dep", 1),
/// ];
/// let bmt = BehavioralMerkleTree::new(symbols);
///
/// if let Some(proof) = bmt.generate_proof("Tf", 0) {
///     let root = bmt.root();
///     assert!(proof.verify(&root));
///
///     // Serialize and deserialize
///     let bytes = proof.serialize();
///     let restored = sods_core::Proof::deserialize(&bytes).unwrap();
///     assert!(restored.verify(&root));
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proof {
    /// The symbol code being proved
    pub symbol: String,

    /// The log index of the symbol
    pub log_index: u32,

    /// The leaf hash (SHA256 of symbol + metadata)
    pub leaf_hash: [u8; 32],

    /// Sibling hashes from leaf to root
    pub path: Vec<[u8; 32]>,

    /// Direction for each sibling: true = sibling is on right, false = on left
    pub directions: Vec<bool>,
}

impl Proof {
    /// Verify this proof against an expected root hash.
    ///
    /// Returns `true` if the proof is valid, `false` otherwise.
    ///
    /// # Arguments
    ///
    /// * `expected_root` - The expected BMT root hash
    ///
    /// # Example
    ///
    /// ```rust
    /// use sods_core::{BehavioralMerkleTree, BehavioralSymbol};
    ///
    /// let symbols = vec![BehavioralSymbol::new("Tf", 0)];
    /// let bmt = BehavioralMerkleTree::new(symbols);
    /// let proof = bmt.generate_proof("Tf", 0).unwrap();
    ///
    /// assert!(proof.verify(&bmt.root()));
    /// ```
    pub fn verify(&self, expected_root: &[u8; 32]) -> bool {
        if self.path.len() != self.directions.len() {
            return false;
        }

        let mut current = self.leaf_hash;

        for (sibling, is_right) in self.path.iter().zip(self.directions.iter()) {
            let mut hasher = Sha256::new();

            if *is_right {
                // Sibling is on right: H(current || sibling)
                hasher.update(current);
                hasher.update(sibling);
            } else {
                // Sibling is on left: H(sibling || current)
                hasher.update(sibling);
                hasher.update(current);
            }

            current = hasher.finalize().into();
        }

        current == *expected_root
    }

    /// Serialize this proof to compact binary format.
    ///
    /// Uses bincode for efficient binary encoding.
    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deserialize a proof from binary format.
    ///
    /// # Errors
    ///
    /// Returns `SodsError::InvalidProof` if the data is malformed.
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        bincode::deserialize(data)
            .map_err(|e| SodsError::InvalidProof(e.to_string()))
    }

    /// Returns the depth of the proof (number of tree levels).
    #[inline]
    pub fn depth(&self) -> usize {
        self.path.len()
    }

    /// Returns the serialized size in bytes.
    pub fn size(&self) -> usize {
        self.serialize().len()
    }
}

/// A proof that a sequence of symbols is causally linked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalProof {
    /// The root of the Causal Merkle Tree
    pub root: [u8; 32],
    /// The symbols in the causal sequence
    pub symbols: Vec<crate::symbol::BehavioralSymbol>,
    /// Merkle proofs for each symbol (proving inclusion in the CMT)
    /// Optimization: Could use a MultiProof, but individual proofs are easier for MPV.
    pub proofs: Vec<Proof>,
}

impl CausalProof {
    /// Verify that the sequence represents a valid causal narrative.
    pub fn verify(&self, expected_root: &[u8; 32]) -> bool {
        // 1. Check Root
        if &self.root != expected_root {
            return false;
        }

        if self.symbols.is_empty() || self.symbols.len() != self.proofs.len() {
            return false;
        }

        // 2. Verify Inclusion (Merkle Proofs)
        for (symbol, proof) in self.symbols.iter().zip(self.proofs.iter()) {
            if proof.leaf_hash != symbol.leaf_hash() {
                return false;
            }
            if !proof.verify(expected_root) {
                return false;
            }
        }

        // 3. Verify Causality (Same Origin + Sequential Nonce/Trace)
        let first = &self.symbols[0];
        let origin = first.from;
        let mut prev_nonce = first.nonce;
        let mut prev_seq = first.call_sequence;

        for (i, sym) in self.symbols.iter().enumerate().skip(1) {
            // Must have same origin
            if sym.from != origin {
                return false;
            }

            // Must be sequential
            // Case A: EOA (Nonce increases by 1)
            let nonce_ok = sym.nonce == prev_nonce + 1;
            
            // Case B: Contrcat (Same Nonce, Sequence increases)
            let seq_ok = sym.nonce == prev_nonce && sym.call_sequence > prev_seq;

            if !nonce_ok && !seq_ok {
                return false;
            }

            prev_nonce = sym.nonce;
            prev_seq = sym.call_sequence;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::BehavioralMerkleTree;
    use crate::symbol::BehavioralSymbol;

    fn create_test_tree() -> BehavioralMerkleTree {
        let symbols = vec![
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Dep", 1),
            BehavioralSymbol::new("Wdw", 2),
            BehavioralSymbol::new("Sw", 3),
        ];
        BehavioralMerkleTree::new(symbols)
    }

    #[test]
    fn test_proof_verification() {
        let bmt = create_test_tree();
        let root = bmt.root();

        for (symbol, log_idx) in [("Tf", 0), ("Dep", 1), ("Wdw", 2), ("Sw", 3)] {
            let proof = bmt.generate_proof(symbol, log_idx).unwrap();
            assert!(proof.verify(&root), "Proof for {} should verify", symbol);
        }
    }

    #[test]
    fn test_proof_verification_wrong_root() {
        let bmt = create_test_tree();
        let proof = bmt.generate_proof("Tf", 0).unwrap();

        let wrong_root = [0u8; 32];
        assert!(!proof.verify(&wrong_root));
    }

    #[test]
    fn test_proof_serialization_roundtrip() {
        let bmt = create_test_tree();
        let proof = bmt.generate_proof("Dep", 1).unwrap();

        let bytes = proof.serialize();
        let restored = Proof::deserialize(&bytes).unwrap();

        assert_eq!(proof.symbol, restored.symbol);
        assert_eq!(proof.log_index, restored.log_index);
        assert_eq!(proof.leaf_hash, restored.leaf_hash);
        assert_eq!(proof.path, restored.path);
        assert_eq!(proof.directions, restored.directions);

        // Restored proof should still verify
        assert!(restored.verify(&bmt.root()));
    }

    #[test]
    fn test_proof_size() {
        let bmt = create_test_tree();
        let proof = bmt.generate_proof("Tf", 0).unwrap();

        let size = proof.size();
        assert!(size > 0);
        assert!(size < 1000); // Should be compact
    }

    #[test]
    fn test_invalid_deserialization() {
        let result = Proof::deserialize(&[1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_proof() {
        let bmt = create_test_tree();
        let mut proof = bmt.generate_proof("Tf", 0).unwrap();

        // Tamper with a path element
        if !proof.path.is_empty() {
            proof.path[0][0] ^= 0xFF;
        }

        assert!(!proof.verify(&bmt.root()));
    }

    #[test]
    fn test_single_leaf_proof() {
        let symbols = vec![BehavioralSymbol::new("Tf", 0)];
        let bmt = BehavioralMerkleTree::new(symbols);
        let root = bmt.root();

        let proof = bmt.generate_proof("Tf", 0).unwrap();
        assert_eq!(proof.depth(), 0); // No siblings needed
        assert!(proof.verify(&root));
    }
}
