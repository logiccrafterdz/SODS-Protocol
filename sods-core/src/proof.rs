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
        
        let mut data = Vec::with_capacity(384);

        // 1. blockNumber (uint256)
        let mut block_bytes = [0u8; 32];
        block_bytes[24..32].copy_from_slice(&self.block_number.to_be_bytes());
        data.extend_from_slice(&block_bytes);

        // 2. chainId (uint256)
        let mut chain_bytes = [0u8; 32];
        chain_bytes[24..32].copy_from_slice(&self.chain_id.to_be_bytes());
        data.extend_from_slice(&chain_bytes);

        // PLACEHOLDERS for dynamic offsets (3, 4, 5, 6, 11)
        for _ in 0..4 { data.extend_from_slice(&[0u8; 32]); } // 3, 4, 5, 6

        // 7. bmtRoot (bytes32)
        data.extend_from_slice(&self.bmt_root);

        // 8. beaconRoot (bytes32)
        data.extend_from_slice(&self.beacon_root.unwrap_or([0u8; 32]));

        // 9. timestamp (uint256)
        let mut ts_bytes = [0u8; 32];
        ts_bytes[24..32].copy_from_slice(&self.timestamp.to_be_bytes());
        data.extend_from_slice(&ts_bytes);

        // 10. receiptsRoot (bytes32)
        data.extend_from_slice(&self.receipts_root.unwrap_or([0u8; 32]));

        // 11. signature (bytes) OFFSET (placeholder)
        data.extend_from_slice(&[0u8; 32]);

        // 12. trustedSigner (address)
        data.extend_from_slice(&[0u8; 32]); // Place holder for signer address (handled off-chain or ignored if not used)

        self.encode_with_dynamic_data(data)
    }

    fn encode_with_dynamic_data(&self, mut base_data: Vec<u8>) -> Vec<u8> {
        let dynamic_start = 384; // 12 slots * 32
        let mut current_offset = dynamic_start;

        // Offset 3 (symbols) at index 64
        let mut symbols_offset = [0u8; 32];
        symbols_offset[31] = (current_offset % 256) as u8;
        symbols_offset[30] = (current_offset / 256) as u8;
        base_data[64..96].copy_from_slice(&symbols_offset);
        let symbols_data = self.encode_string_array();
        current_offset += symbols_data.len();

        // Offset 4 (logIndices) at index 96
        let mut logs_offset = [0u8; 32];
        logs_offset[31] = (current_offset % 256) as u8;
        logs_offset[30] = (current_offset / 256) as u8;
        base_data[96..128].copy_from_slice(&logs_offset);
        let logs_data = self.encode_uint32_array();
        current_offset += logs_data.len();

        // Offset 5 (leafHashes) at index 128
        let mut leaves_offset = [0u8; 32];
        leaves_offset[31] = (current_offset % 256) as u8;
        leaves_offset[30] = (current_offset / 256) as u8;
        base_data[128..160].copy_from_slice(&leaves_offset);
        let leaves_data = self.encode_bytes32_array(&self.leaf_hashes);
        current_offset += leaves_data.len();

        // Offset 6 (merklePath) at index 160
        let mut path_offset = [0u8; 32];
        path_offset[31] = (current_offset % 256) as u8;
        path_offset[30] = (current_offset / 256) as u8;
        base_data[160..192].copy_from_slice(&path_offset);
        let path_data = self.encode_bytes32_array(&self.merkle_path);
        current_offset += path_data.len();

        // Offset 11 (signature) at index 320 (10th slot)
        let mut sig_offset = [0u8; 32];
        sig_offset[31] = (current_offset % 256) as u8;
        sig_offset[30] = (current_offset / 256) as u8;
        base_data[320..352].copy_from_slice(&sig_offset);
        
        let mut sig_data = Vec::new();
        if let Some(sig) = &self.signature {
            let mut sig_len = [0u8; 32];
            sig_len[31] = sig.len() as u8;
            sig_data.extend_from_slice(&sig_len);
            sig_data.extend_from_slice(sig);
            while sig_data.len() % 32 != 0 { sig_data.push(0); }
        } else {
            sig_data.extend_from_slice(&[0u8; 32]); // Length 0
        }

        // Append all dynamic data
        base_data.extend_from_slice(&symbols_data);
        base_data.extend_from_slice(&logs_data);
        base_data.extend_from_slice(&leaves_data);
        base_data.extend_from_slice(&path_data);
        base_data.extend_from_slice(&sig_data);

        base_data
    }

    fn encode_string_array(&self) -> Vec<u8> {
        let mut data = Vec::new();
        // Array length
        let mut len_bytes = [0u8; 32];
        len_bytes[31] = self.symbols.len() as u8;
        data.extend_from_slice(&len_bytes);

        // For string[], each element has an offset
        let mut offset = self.symbols.len() * 32;
        let mut dynamic_data = Vec::new();

        for s in &self.symbols {
            let mut off_bytes = [0u8; 32];
            off_bytes[31] = offset as u8;
            data.extend_from_slice(&off_bytes);

            // String data: length then content padded to 32 bytes
            let mut str_len = [0u8; 32];
            str_len[31] = s.len() as u8;
            dynamic_data.extend_from_slice(&str_len);
            
            let mut content = s.as_bytes().to_vec();
            while content.len() % 32 != 0 { content.push(0); }
            dynamic_data.extend_from_slice(&content);
            
            offset += 32 + content.len();
        }
        data.extend_from_slice(&dynamic_data);
        data
    }

    fn encode_uint32_array(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let mut len_bytes = [0u8; 32];
        len_bytes[31] = self.log_indices.len() as u8;
        data.extend_from_slice(&len_bytes);

        for &idx in &self.log_indices {
            let mut val = [0u8; 32];
            val[31] = (idx % 256) as u8;
            val[30] = (idx / 256) as u8;
            data.extend_from_slice(&val);
        }
        data
    }

    fn encode_bytes32_array(&self, arr: &[[u8; 32]]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut len_bytes = [0u8; 32];
        len_bytes[31] = arr.len() as u8;
        data.extend_from_slice(&len_bytes);

        for &val in arr {
            data.extend_from_slice(&val);
        }
        data
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
