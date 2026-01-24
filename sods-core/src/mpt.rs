use ethers_core::types::{H256, Bytes};
use ethers_core::utils::rlp::{self, Rlp};
use sha3::{Digest, Keccak256};
use crate::error::SodsError;

/// Lightweight Merkle-Patricia Trie (MPT) proof verifier.
pub struct MptVerifier;

impl MptVerifier {
    /// Verifies an MPT inclusion proof.
    ///
    /// # Arguments
    /// * `root` - The expected trie root hash.
    /// * `path` - The nibbles of the key being verified.
    /// * `value` - The expected RLP-encoded value at the path.
    /// * `nodes` - The list of RLP-encoded nodes in the proof.
    pub fn verify_proof(
        root: H256,
        path: &[u8],
        value: Option<&[u8]>,
        nodes: &[Bytes],
    ) -> Result<bool, SodsError> {
        let mut current_hash = root;
        let mut nibbles = Self::to_nibbles(path);
        let mut nibble_index = 0;

        for (i, node_bytes) in nodes.iter().enumerate() {
            let node_hash = H256::from_slice(&Keccak256::digest(node_bytes));
            
            // The first node must match the root (unless it's the only node and root is a literal?)
            // Actually, in many implementations the root is the hash of the first node.
            // If i == 0, current_hash == root.
            if node_hash != current_hash {
                return Ok(false);
            }

            let rlp = Rlp::new(node_bytes);
            if !rlp.is_list() {
                return Err(SodsError::InternalError("Invalid MPT node: not a list".into()));
            }

            match rlp.item_count().map_err(|_| SodsError::InternalError("RLP error".into()))? {
                17 => {
                    // Branch node
                    if nibble_index == nibbles.len() {
                        // End of path, check value in the last slot
                        let branch_value = rlp.at(16).map_err(|_| SodsError::InternalError("RLP error".into()))?.data().map_err(|_| SodsError::InternalError("RLP error".into()))?;
                        return Ok(value == Some(branch_value));
                    }
                    let nibble = nibbles[nibble_index] as usize;
                    let next_node_rlp = rlp.at(nibble).map_err(|_| SodsError::InternalError("RLP error".into()))?;
                    
                    if next_node_rlp.is_empty() {
                        return Ok(value.is_none());
                    }
                    
                    current_hash = H256::from_slice(next_node_rlp.data().map_err(|_| SodsError::InternalError("RLP error".into()))?);
                    nibble_index += 1;
                }
                2 => {
                    // Extension or Leaf node
                    let encoded_path = rlp.at(0).map_err(|_| SodsError::InternalError("RLP error".into()))?.data().map_err(|_| SodsError::InternalError("RLP error".into()))?;
                    let (node_nibbles, is_leaf) = Self::decode_path(encoded_path);
                    
                    // Match prefix
                    if nibbles[nibble_index..].starts_with(&node_nibbles) {
                        nibble_index += node_nibbles.len();
                        
                        if is_leaf {
                            if nibble_index == nibbles.len() {
                                let leaf_value = rlp.at(1).map_err(|_| SodsError::InternalError("RLP error".into()))?.data().map_err(|_| SodsError::InternalError("RLP error".into()))?;
                                return Ok(value == Some(leaf_value));
                            } else {
                                return Ok(false); // Leaf reached but path not exhausted
                            }
                        } else {
                            // Extension node
                            let next_node_rlp = rlp.at(1).map_err(|_| SodsError::InternalError("RLP error".into()))?;
                            current_hash = H256::from_slice(next_node_rlp.data().map_err(|_| SodsError::InternalError("RLP error".into()))?);
                        }
                    } else {
                        return Ok(false); // Prefix mismatch
                    }
                }
                _ => return Err(SodsError::InternalError("Invalid MPT node: unexpected item count".into())),
            }
        }

        Ok(false)
    }

    fn to_nibbles(path: &[u8]) -> Vec<u8> {
        let mut nibbles = Vec::with_capacity(path.len() * 2);
        for &byte in path {
            nibbles.push(byte >> 4);
            nibbles.push(byte & 0x0F);
        }
        nibbles
    }

    fn decode_path(encoded: &[u8]) -> (Vec<u8>, bool) {
        if encoded.is_empty() { return (vec![], false); }
        let prefix = encoded[0] >> 4;
        let is_leaf = (prefix & 2) != 0;
        let has_odd_len = (prefix & 1) != 0;
        
        let mut nibbles = Vec::new();
        if has_odd_len {
            nibbles.push(encoded[0] & 0x0F);
        }
        for &byte in &encoded[1..] {
            nibbles.push(byte >> 4);
            nibbles.push(byte & 0x0F);
        }
        (nibbles, is_leaf)
    }
}
