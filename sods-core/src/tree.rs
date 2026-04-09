//! Behavioral Merkle Tree implementation.
//!
//! This module provides the `BehavioralMerkleTree` which constructs a binary
//! Merkle tree over sorted behavioral symbols and supports proof generation.
//!
//! All hashing uses **Keccak256** for EVM compatibility with `SODSVerifier.sol`.

use tiny_keccak::Hasher;

use crate::proof::Proof;
use crate::symbol::BehavioralSymbol;

/// Compute the Keccak256 hash of empty input.
fn keccak256_empty() -> [u8; 32] {
    let hasher = tiny_keccak::Keccak::v256();
    let mut root = [0u8; 32];
    hasher.finalize(&mut root);
    root
}

/// A binary Merkle tree over behavioral symbols.
///
/// **Note**: This is a Behavioral Merkle Tree (BMT), which sorts symbols by log index.
/// This is NOT a Causal Merkle Tree (CMT). See `sods-causal` for actor-based ordering.
///
/// The tree is constructed from a sorted list of `BehavioralSymbol` instances.
/// Symbols are automatically sorted by canonical ordering (log_index, then symbol)
/// before tree construction.
///
/// # Hashing Rules
///
/// - **Leaf**: `Keccak256(symbol_bytes || BigEndian_u32(log_index))`
/// - **Internal node**: `Keccak256(left_hash || right_hash)`
/// - **Empty tree**: `Keccak256(b"")` = `c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470`
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
    /// Uses Keccak256 hashing for EVM compatibility.
    /// Symbols are sorted by canonical ordering before tree construction.
    ///
    /// # Arguments
    ///
    /// * `symbols` - List of behavioral symbols to include in the tree
    pub fn new(mut symbols: Vec<BehavioralSymbol>) -> Self {
        // Sort symbols canonically (by log_index, then symbol)
        symbols.sort();

        if symbols.is_empty() {
            // Empty tree: root = Keccak256(b"")
            let root = keccak256_empty();
            return Self {
                symbols,
                layers: vec![],
                root,
            };
        }

        // Compute leaf hashes (Keccak256)
        let leaves: Vec<[u8; 32]> = symbols.iter().map(|s| s.leaf_hash()).collect();

        // Build tree
        let (layers, root) = Self::build_tree(leaves);

        Self {
            symbols,
            layers,
            root,
        }
    }

    /// Build a Behavioral Merkle Tree from a pre-filtered subset of symbols.
    ///
    /// Use this when you have already filtered symbols (e.g., via topic-based
    /// RPC queries) and want a BMT over just the matched subset.
    /// The resulting tree root commits ONLY to the provided symbols.
    pub fn from_filtered(symbols: Vec<BehavioralSymbol>) -> Self {
        Self::new(symbols)
    }

    /// Build the Merkle tree layers from leaves to root using Keccak256.
    fn build_tree(leaves: Vec<[u8; 32]>) -> (Vec<Vec<[u8; 32]>>, [u8; 32]) {
        if leaves.is_empty() {
            return (vec![], keccak256_empty());
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

                // Parent = Keccak256(left || right)
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
        let leaf_index = self
            .symbols
            .iter()
            .position(|s| s.symbol() == symbol && s.log_index() == log_index)?;

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
            leaf_hashes.push(s.leaf_hash());
        }

        // For simplicity in the first version, we'll provide the proof for the FIRST symbol
        let first_idx = self.symbols.iter().position(|s| {
            s.symbol() == matched_symbols[0].symbol()
                && s.log_index() == matched_symbols[0].log_index()
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

        // Empty tree root = Keccak256(b"")
        let expected = keccak256_empty();
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

        // Root = Keccak256(leaf0 || leaf1)
        let leaf0 = BehavioralSymbol::new("Tf", 0).leaf_hash();
        let leaf1 = BehavioralSymbol::new("Dep", 1).leaf_hash();

        let mut hasher = tiny_keccak::Keccak::v256();
        hasher.update(&leaf0);
        hasher.update(&leaf1);
        let mut expected = [0u8; 32];
        hasher.finalize(&mut expected);

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

    #[test]
    fn test_leaf_hash_matches_solidity_abi_encode_packed() {
        // Verify that our leaf_hash matches keccak256(abi.encodePacked(symbol, logIndex))
        let sym = BehavioralSymbol::new("Tf", 42);
        let rust_hash = sym.leaf_hash();

        // Manually compute keccak256(abi.encodePacked("Tf", uint32(42)))
        let mut input = Vec::new();
        input.extend_from_slice(b"Tf");
        input.extend_from_slice(&42u32.to_be_bytes());

        let mut hasher = tiny_keccak::Keccak::v256();
        hasher.update(&input);
        let mut expected = [0u8; 32];
        hasher.finalize(&mut expected);

        assert_eq!(
            rust_hash, expected,
            "Leaf hash must match Solidity ABI encoding"
        );
    }

    #[test]
    fn test_from_filtered_equals_new() {
        let symbols = vec![
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Dep", 1),
        ];

        let bmt_new = BehavioralMerkleTree::new(symbols.clone());
        let bmt_filtered = BehavioralMerkleTree::from_filtered(symbols);

        assert_eq!(bmt_new.root(), bmt_filtered.root());
    }
}
