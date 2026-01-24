//! Behavioral Merkle Tree implementation.
//!
//! This module provides the `BehavioralMerkleTree` which constructs a binary
//! Merkle tree over sorted behavioral symbols and supports proof generation.

use sha2::{Digest, Sha256};
use tiny_keccak::Hasher;

use crate::proof::Proof;
use crate::symbol::BehavioralSymbol;

/// A binary Merkle tree over behavioral symbols.
///
/// The tree is constructed from a sorted list of `BehavioralSymbol` instances.
/// Symbols are automatically sorted by canonical ordering (log_index, then symbol)
/// before tree construction.
///
/// # Hashing Rules (per RFC §4.1)
///
/// - **Leaf**: `SHA256(symbol_bytes || metadata)` (minimal mode uses symbol only)
/// - **Internal node**: `SHA256(left_hash || right_hash)`
/// - **Empty tree**: `SHA256(b"")` = `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
/// - **Odd leaves**: last node is duplicated
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
///
/// let bmt = BehavioralMerkleTree::new(symbols);
/// let root = bmt.root();
/// println!("BMT Root: 0x{}", hex::encode(root));
/// ```
#[derive(Debug, Clone)]
pub struct BehavioralMerkleTree {
    /// Sorted symbols
    symbols: Vec<BehavioralSymbol>,

    /// Tree layers, from leaves (index 0) to root (last index)
    layers: Vec<Vec<[u8; 32]>>,

    /// The root hash
    root: [u8; 32],
}

impl BehavioralMerkleTree {
    /// Build a new Behavioral Merkle Tree from a list of symbols.
    ///
    /// Symbols are sorted by canonical ordering before tree construction.
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of behavioral symbols to include in the tree
    pub fn new(mut symbols: Vec<BehavioralSymbol>) -> Self {
        // Sort symbols canonically (by log_index, then symbol)
        symbols.sort();

        if symbols.is_empty() {
            // Empty tree: root = SHA256(b"")
            let root = Sha256::digest([]).into();
            return Self {
                symbols,
                layers: vec![],
                root,
            };
        }

        // Compute leaf hashes
        let leaves: Vec<[u8; 32]> = symbols.iter().map(|s| s.leaf_hash()).collect();

        // Build tree
        let (layers, root) = Self::build_tree(leaves);

        Self {
            symbols,
            layers,
            root,
        }
    }

    /// Build an incremental Behavioral Merkle Tree from a pre-filtered list of symbols.
    ///
    /// This is optimized for pattern matching where only a subset of block logs are fetched.
    /// The resulting tree is smaller and its root commits ONLY to the provided symbols.
    pub fn build_incremental(symbols: Vec<BehavioralSymbol>) -> Self {
        // Since symbols are already filtered, we just use the existing constructor logic.
        // The term "incremental" here denotes that it builds a partial view of the block's behavior.
        Self::new(symbols)
    }

    /// Build the Merkle tree layers from leaves to root.
    fn build_tree(leaves: Vec<[u8; 32]>) -> (Vec<Vec<[u8; 32]>>, [u8; 32]) {
        if leaves.is_empty() {
            return (vec![], Sha256::digest([]).into());
        }

        if leaves.len() == 1 {
            return (vec![leaves.clone()], leaves[0]);
        }

        let mut layers = vec![leaves];

        loop {
            let current = layers.last().unwrap();

            if current.len() == 1 {
                break;
            }

            let mut next_layer = Vec::with_capacity((current.len() + 1) / 2);

            for i in (0..current.len()).step_by(2) {
                let left = current[i];

                // If odd number of nodes, duplicate the last one
                let right = if i + 1 < current.len() {
                    current[i + 1]
                } else {
                    left
                };

                // Parent = H(left || right)
                let mut hasher = Sha256::new();
                hasher.update(left);
                hasher.update(right);
                let parent: [u8; 32] = hasher.finalize().into();

                next_layer.push(parent);
            }

            layers.push(next_layer);
        }

        let root = layers.last().unwrap()[0];
        (layers, root)
    }

    /// Build a new Behavioral Merkle Tree using Keccak256 hashing.
    pub fn new_keccak(mut symbols: Vec<BehavioralSymbol>) -> Self {
        symbols.sort();

        if symbols.is_empty() {
            let hasher = tiny_keccak::Keccak::v256();
            let mut root = [0u8; 32];
            hasher.finalize(&mut root);
            return Self {
                symbols,
                layers: vec![],
                root,
            };
        }

        let leaves: Vec<[u8; 32]> = symbols.iter().map(|s| s.leaf_hash_keccak()).collect();
        let (layers, root) = Self::build_tree_keccak(leaves);

        Self {
            symbols,
            layers,
            root,
        }
    }

    /// Build the Merkle tree layers using Keccak256.
    fn build_tree_keccak(leaves: Vec<[u8; 32]>) -> (Vec<Vec<[u8; 32]>>, [u8; 32]) {
        if leaves.is_empty() {
            let hasher = tiny_keccak::Keccak::v256();
            let mut root = [0u8; 32];
            hasher.finalize(&mut root);
            return (vec![], root);
        }

        if leaves.len() == 1 {
            return (vec![leaves.clone()], leaves[0]);
        }

        let mut layers = vec![leaves];

        loop {
            let current = layers.last().unwrap();
            if current.len() == 1 { break; }

            let mut next_layer = Vec::with_capacity((current.len() + 1) / 2);

            for i in (0..current.len()).step_by(2) {
                let left = current[i];
                let right = if i + 1 < current.len() { current[i + 1] } else { left };

                let mut hasher = tiny_keccak::Keccak::v256();
                hasher.update(&left);
                hasher.update(&right);
                let mut parent = [0u8; 32];
                hasher.finalize(&mut parent);

                next_layer.push(parent);
            }

            layers.push(next_layer);
        }

        let root = layers.last().unwrap()[0];
        (layers, root)
    }

    /// Returns the root hash of the tree.
    ///
    /// This is the 32-byte cryptographic commitment to all symbols in the tree.
    #[inline]
    pub fn root(&self) -> [u8; 32] {
        self.root
    }

    /// Returns the number of symbols in the tree.
    #[inline]
    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    /// Returns true if the tree is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }

    /// Returns a reference to the sorted symbols.
    pub fn symbols(&self) -> &[BehavioralSymbol] {
        &self.symbols
    }

    /// Generate a Merkle inclusion proof for a specific symbol.
    ///
    /// Returns `None` if the symbol is not found at the specified log_index.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol code to prove (e.g., "Tf")
    /// * `log_index` - The log index where the symbol should be
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
    ///
    /// let bmt = BehavioralMerkleTree::new(symbols);
    ///
    /// if let Some(proof) = bmt.generate_proof("Tf", 0) {
    ///     assert!(proof.verify(&bmt.root()));
    /// }
    /// ```
    pub fn generate_proof(&self, symbol: &str, log_index: u32) -> Option<Proof> {
        // Find the symbol in the sorted list
        let leaf_index = self.symbols.iter().position(|s| {
            s.symbol() == symbol && s.log_index() == log_index
        })?;

        self.generate_proof_by_index(leaf_index)
    }

    /// Generate a Merkle proof by leaf index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn generate_proof_by_index(&self, leaf_index: usize) -> Option<Proof> {
        if leaf_index >= self.symbols.len() || self.layers.is_empty() {
            return None;
        }

        let symbol = &self.symbols[leaf_index];
        let leaf_hash = self.layers[0][leaf_index];

        let mut path = Vec::new();
        let mut directions = Vec::new();
        let mut idx = leaf_index;

        // Traverse from leaf layer (0) up to second-to-last layer
        for layer in self.layers.iter().take(self.layers.len().saturating_sub(1)) {
            if idx % 2 == 0 {
                // Current is left child, sibling is right
                let sibling_idx = idx + 1;
                if sibling_idx < layer.len() {
                    path.push(layer[sibling_idx]);
                } else {
                    // Odd layer, duplicate self
                    path.push(layer[idx]);
                }
                directions.push(true); // sibling is on right
            } else {
                // Current is right child, sibling is left
                let sibling_idx = idx - 1;
                path.push(layer[sibling_idx]);
                directions.push(false); // sibling is on left
            }

            // Move to parent index
            idx /= 2;
        }

        Some(Proof {
            symbol: symbol.symbol().to_string(),
            log_index: symbol.log_index(),
            leaf_hash,
            path,
            directions,
        })
    }

    /// Generate an on-chain verifiable proof.
    pub fn generate_onchain_proof(
        &self, 
        matched_symbols: &[&BehavioralSymbol], 
        chain_id: u64, 
        block_number: u64,
        beacon_root: Option<[u8; 32]>,
        timestamp: u64,
    ) -> Option<crate::proof::OnChainBehavioralProof> {
        let mut symbols = Vec::new();
        let mut log_indices = Vec::new();
        let mut leaf_hashes = Vec::new();
        
        for s in matched_symbols {
            symbols.push(s.symbol().to_string());
            log_indices.push(s.log_index());
            leaf_hashes.push(s.leaf_hash_keccak());
        }

        // For simplicity in the first version, we'll provide the proof for the FIRST symbol
        let first_idx = self.symbols.iter().position(|s| {
            s.symbol() == matched_symbols[0].symbol() && s.log_index() == matched_symbols[0].log_index()
        })?;

        let proof = self.generate_proof_by_index(first_idx)?;

        Some(crate::proof::OnChainBehavioralProof {
            block_number,
            chain_id,
            symbols,
            log_indices,
            leaf_hashes,
            merkle_path: proof.path,
            is_left_path: proof.directions,
            bmt_root: self.root,
            beacon_root,
            timestamp,
            receipts_root: None,
            signature: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let bmt = BehavioralMerkleTree::new(vec![]);

        // Empty tree root = SHA256(b"")
        let expected: [u8; 32] = Sha256::digest([]).into();
        assert_eq!(bmt.root(), expected);
        assert!(bmt.is_empty());
    }

    #[test]
    fn test_single_leaf() {
        let symbols = vec![BehavioralSymbol::new("Tf", 0)];
        let bmt = BehavioralMerkleTree::new(symbols);

        // Single leaf: root = leaf_hash
        let expected = BehavioralSymbol::new("Tf", 0).leaf_hash();
        assert_eq!(bmt.root(), expected);
        assert_eq!(bmt.len(), 1);
    }

    #[test]
    fn test_two_leaves() {
        let symbols = vec![
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Dep", 1),
        ];
        let bmt = BehavioralMerkleTree::new(symbols);

        // Root = H(leaf0 || leaf1)
        let leaf0 = BehavioralSymbol::new("Tf", 0).leaf_hash();
        let leaf1 = BehavioralSymbol::new("Dep", 1).leaf_hash();

        let mut hasher = Sha256::new();
        hasher.update(leaf0);
        hasher.update(leaf1);
        let expected: [u8; 32] = hasher.finalize().into();

        assert_eq!(bmt.root(), expected);
    }

    #[test]
    fn test_canonical_sorting() {
        // Input out of order
        let symbols = vec![
            BehavioralSymbol::new("Wdw", 10),
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Dep", 5),
        ];
        let bmt = BehavioralMerkleTree::new(symbols);

        // Should be sorted
        assert_eq!(bmt.symbols()[0].log_index(), 0);
        assert_eq!(bmt.symbols()[1].log_index(), 5);
        assert_eq!(bmt.symbols()[2].log_index(), 10);
    }

    #[test]
    fn test_proof_generation() {
        let symbols = vec![
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Dep", 1),
            BehavioralSymbol::new("Wdw", 2),
        ];
        let bmt = BehavioralMerkleTree::new(symbols);

        let proof = bmt.generate_proof("Dep", 1).expect("Proof should exist");
        assert_eq!(proof.symbol, "Dep");
        assert_eq!(proof.log_index, 1);
        assert!(proof.verify(&bmt.root()));
    }

    #[test]
    fn test_proof_not_found() {
        let symbols = vec![BehavioralSymbol::new("Tf", 0)];
        let bmt = BehavioralMerkleTree::new(symbols);

        assert!(bmt.generate_proof("Dep", 0).is_none());
        assert!(bmt.generate_proof("Tf", 99).is_none());
    }

    #[test]
    fn test_odd_number_of_leaves() {
        let symbols = vec![
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Dep", 1),
            BehavioralSymbol::new("Wdw", 2),
        ];
        let bmt = BehavioralMerkleTree::new(symbols);

        // All proofs should verify
        assert!(bmt.generate_proof("Tf", 0).unwrap().verify(&bmt.root()));
        assert!(bmt.generate_proof("Dep", 1).unwrap().verify(&bmt.root()));
        assert!(bmt.generate_proof("Wdw", 2).unwrap().verify(&bmt.root()));
    }
}
