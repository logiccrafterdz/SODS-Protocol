//! Causal Merkle Tree implementation for agent behavior verification.
//!
//! This module provides the `CausalMerkleTree` which constructs a binary
//! Merkle tree over causally-ordered events and supports proof generation.

use ethers::types::H256;
use tiny_keccak::{Hasher, Keccak};
use crate::event::CausalEvent;
use crate::proof::CausalProof;
use crate::error::{CausalError, Result};

/// A cryptographically secure Merkle tree for causal events.
///
/// The tree is built over a list of `CausalEvent`s which must be
/// strictly ordered by (nonce, sequence_index).
#[derive(Debug, Clone)]
pub struct CausalMerkleTree {
    /// The root hash of the tree.
    pub root: H256,
    /// All levels of the tree, from leaves to root.
    levels: Vec<Vec<H256>>,
    /// The events included in the tree, in causal order.
    events: Vec<CausalEvent>,
}

impl CausalMerkleTree {
    /// Constructs a new `CausalMerkleTree` from a list of events.
    ///
    /// # Errors
    /// Returns `CausalError` if the events are not strictly ordered.
    pub fn new(events: Vec<CausalEvent>) -> Result<Self> {
        if events.is_empty() {
            return Ok(Self {
                root: H256::zero(),
                levels: vec![],
                events: vec![],
            });
        }

        // Verify causal ordering
        for i in 0..events.len().saturating_sub(1) {
            if events[i] >= events[i + 1] {
                return Err(CausalError::SequenceGap {
                    expected: 0, // Simplified for error message
                    actual: 0,
                });
            }
        }

        // Compute leaf hashes: Keccak256(RLP(event))
        let leaves: Vec<H256> = events
            .iter()
            .map(|event| {
                let serialized = event.rlp_encode();
                let mut hash = [0u8; 32];
                let mut hasher = Keccak::v256();
                hasher.update(&serialized);
                hasher.finalize(&mut hash);
                H256::from(hash)
            })
            .collect();

        let (levels, root) = Self::build_tree(leaves);

        Ok(Self {
            root,
            levels,
            events,
        })
    }

    /// Builds the Merkle tree from leaves.
    fn build_tree(leaves: Vec<H256>) -> (Vec<Vec<H256>>, H256) {
        let mut levels = vec![leaves];

        while levels.last().unwrap().len() > 1 {
            let next_level = Self::compute_next_level(levels.last().unwrap());
            levels.push(next_level);
        }

        let root = levels.last().unwrap()[0];
        (levels, root)
    }

    /// Computes the next level of the Merkle tree.
    fn compute_next_level(current_level: &[H256]) -> Vec<H256> {
        let mut next_level = Vec::with_capacity((current_level.len() + 1) / 2);

        for chunk in current_level.chunks(2) {
            let left = chunk[0];
            let right = if chunk.len() > 1 { chunk[1] } else { left };

            let mut hash = [0u8; 32];
            let mut hasher = Keccak::v256();
            hasher.update(left.as_bytes());
            hasher.update(right.as_bytes());
            hasher.finalize(&mut hash);
            next_level.push(H256::from(hash));
        }

        next_level
    }

    /// Generates an inclusion proof for an event at the given index.
    ///
    /// # Panics
    /// Panics if the index is out of bounds.
    pub fn generate_proof(&self, event_index: usize) -> CausalProof {
        if event_index >= self.events.len() {
            panic!("Event index out of bounds");
        }

        let mut merkle_path = Vec::new();
        let mut is_left_path = Vec::new();
        let mut index = event_index;

        for level in &self.levels {
            if level.len() <= 1 {
                break;
            }

            let sibling_index = if index % 2 == 0 {
                // index is left, sibling is right
                index + 1
            } else {
                // index is right, sibling is left
                index - 1
            };

            let sibling_hash = if sibling_index < level.len() {
                level[sibling_index]
            } else {
                level[index]
            };

            merkle_path.push(sibling_hash);
            is_left_path.push(index % 2 != 0); // If current is right, sibling is left

            index /= 2;
        }

        CausalProof {
            event: self.events[event_index].clone(),
            merkle_path,
            is_left_path,
            root: self.root,
        }
    }

    /// Returns the events in the tree.
    pub fn events(&self) -> &[CausalEvent] {
        &self.events
    }

    /// Returns the number of levels in the tree.
    pub fn levels_len(&self) -> usize {
        self.levels.len()
    }

    /// Returns the hash at a specific level and index.
    pub fn get_hash(&self, level: usize, index: usize) -> Option<H256> {
        self.levels.get(level)?.get(index).cloned()
    }

    /// Returns a reference to a specific level of the tree.
    /// This is primarily for testing purposes.
    pub fn get_level(&self, level_index: usize) -> Option<&Vec<H256>> {
        self.levels.get(level_index)
    }
}
