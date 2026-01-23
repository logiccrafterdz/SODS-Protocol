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

/// A behavioral proof optimized for on-chain verification in Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainBehavioralProof {
    /// Block number
    pub block_number: u64,
    /// Chain ID
    pub chain_id: u64,
    /// The sequence of symbols being proved
    pub symbols: Vec<String>,
    /// The log indices of those symbols
    pub log_indices: Vec<u32>,
    /// The Keccak256 leaf hashes
    pub leaf_hashes: Vec<[u8; 32]>,
    /// The shared Merkle path
    pub merkle_path: Vec<[u8; 32]>,
    /// The BMT root (Keccak256)
    pub bmt_root: [u8; 32],
    /// Beacon root for the block (EIP-4788)
    pub beacon_root: Option<[u8; 32]>,
    /// Block timestamp (for beacon root lookup)
    pub timestamp: u64,
    /// Receipts root (for signed commitment)
    pub receipts_root: Option<[u8; 32]>,
    /// ECDSA signature (optional)
    pub signature: Option<Vec<u8>>,
}

impl OnChainBehavioralProof {
    /// Export the proof as ABI-encoded calldata for `SODSVerifier.verifyBehavior`.
    pub fn to_calldata(&self) -> Vec<u8> {
        // signature: verifyBehavior(uint256,uint256,string[],uint32[],bytes32[],bytes32[],bytes32,bytes32,uint256,bytes32,bytes,address)
        use ethabi::{encode, Token};

        let tokens = vec![
            Token::Uint(self.block_number.into()),
            Token::Uint(self.chain_id.into()),
            Token::Array(
                self.symbols
                    .iter()
                    .map(|s| Token::String(s.clone()))
                    .collect(),
            ),
            Token::Array(
                self.log_indices
                    .iter()
                    .map(|&i| Token::Uint(i.into()))
                    .collect(),
            ),
            Token::Array(
                self.leaf_hashes
                    .iter()
                    .map(|&h| Token::FixedBytes(h.to_vec()))
                    .collect(),
            ),
            Token::Array(
                self.merkle_path
                    .iter()
                    .map(|&h| Token::FixedBytes(h.to_vec()))
                    .collect(),
            ),
            Token::FixedBytes(self.bmt_root.to_vec()),
            Token::FixedBytes(self.beacon_root.unwrap_or([0u8; 32]).to_vec()),
            Token::Uint(self.timestamp.into()),
            Token::FixedBytes(self.receipts_root.unwrap_or([0u8; 32]).to_vec()),
            Token::Bytes(self.signature.clone().unwrap_or_default()),
            Token::Address(ethabi::Address::zero()), // trustedSigner (placeholder)
        ];

        encode(&tokens)
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

        for (_i, sym) in self.symbols.iter().enumerate().skip(1) {
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

    #[test]
    fn test_onchain_proof_manual_abi_serialization() {
        let syms = vec![
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Sw", 1),
        ];
        let bmt = BehavioralMerkleTree::new_keccak(syms.clone());
        let matched = vec![&syms[0], &syms[1]];
        
        let proof = bmt.generate_onchain_proof(&matched, 11155111, 100, None, 1700000000).unwrap();
        let calldata = proof.to_calldata();
        
        // Basic length check for ABI encoded dynamic data
        // 7 slots (224 bytes) base + symbols data + indices data + hashes data + path data
        assert!(calldata.len() > 224);
        assert_eq!(calldata.len() % 32, 0); // ABI encoding is always 32-byte padded
        
        println!("Calldata len: {}", calldata.len());
        println!("Calldata: 0x{}", hex::encode(&calldata));
    }
}
