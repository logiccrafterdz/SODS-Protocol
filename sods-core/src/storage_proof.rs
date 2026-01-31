use ethers_core::types::{H256, EIP1186ProofResponse};
use ethers_core::utils::rlp::Rlp;
use sha3::{Digest, Keccak256};

/// Verification result for a storage proof.
#[derive(Debug, Clone, PartialEq)]
pub struct ProofValidation {
    pub is_valid: bool,
    pub account_valid: bool,
    pub storage_valid: Vec<bool>,
}

/// Verify an EIP-1186 account and storage proof against a state root.
pub fn verify_storage_proof(
    proof: &EIP1186ProofResponse,
    state_root: H256,
) -> ProofValidation {
    // 1. Verify Account Proof
    let account_valid = verify_mpt_proof(
        state_root,
        &Keccak256::digest(proof.address.as_bytes()).to_vec(),
        &proof.account_proof,
    );

    // 2. Verify Storage Proofs
    let mut storage_valid = Vec::with_capacity(proof.storage_proof.len());
    if account_valid {
        // Extract storage root from account data
        if let Some(account_rlp) = get_leaf_value(state_root, &Keccak256::digest(proof.address.as_bytes()).to_vec(), &proof.account_proof) {
            let rlp = Rlp::new(&account_rlp);
            // Account RLP is [nonce, balance, storageRoot, codeHash]
            if rlp.item_count().unwrap_or(0) >= 3 {
                let storage_root: H256 = rlp.at(2).unwrap().as_val().unwrap();
                
                for sp in &proof.storage_proof {
                    let mut key_bytes = [0u8; 32];
                    sp.key.to_big_endian(&mut key_bytes);
                    let key_hash = Keccak256::digest(&key_bytes).to_vec();
                    let valid = verify_mpt_proof(storage_root, &key_hash, &sp.proof);
                    storage_valid.push(valid);
                }
            } else {
                storage_valid.resize(proof.storage_proof.len(), false);
            }
        } else {
            storage_valid.resize(proof.storage_proof.len(), false);
        }
    } else {
        storage_valid.resize(proof.storage_proof.len(), false);
    }

    ProofValidation {
        is_valid: account_valid && storage_valid.iter().all(|&v| v),
        account_valid,
        storage_valid,
    }
}

/// Verify a Merkle-Patricia Trie proof.
/// 
/// This is a simplified verification that handles the proof path provided by eth_getProof.
pub fn verify_mpt_proof(root: H256, key_hash: &[u8], proof: &[ethers_core::types::Bytes]) -> bool {
    get_leaf_value(root, key_hash, proof).is_some()
}

/// Traverses the MPT proof to extract the leaf value.
fn get_leaf_value(root: H256, key_hash: &[u8], proof: &[ethers_core::types::Bytes]) -> Option<Vec<u8>> {
    let mut expected_hash = root;
    let key_bits = to_nibbles(key_hash);
    let mut key_offset = 0;

    for node_bytes in proof {
        let node_hash = H256::from_slice(&Keccak256::digest(node_bytes));
        if node_hash != expected_hash {
            return None;
        }

        let rlp = Rlp::new(node_bytes);
        match rlp.item_count().unwrap_or(0) {
            2 => {
                // Extension or Leaf node
                let encoded_path = rlp.at(0).ok()?.data().ok()?.to_vec();
                let (is_leaf, path) = decode_path(&encoded_path);
                
                // Compare path with key bits
                if key_bits[key_offset..key_offset + path.len()] != path {
                    return None;
                }
                key_offset += path.len();

                if is_leaf {
                    if key_offset != key_bits.len() {
                        return None;
                    }
                    return Some(rlp.at(1).ok()?.data().ok()?.to_vec());
                } else {
                    expected_hash = rlp.at(1).ok()?.as_val().ok()?;
                }
            }
            17 => {
                // Branch node
                if key_offset == key_bits.len() {
                    let val = rlp.at(16).ok()?.data().ok()?.to_vec();
                    return if val.is_empty() { None } else { Some(val) };
                }
                let nibble = key_bits[key_offset];
                key_offset += 1;
                expected_hash = rlp.at(nibble as usize).ok()?.as_val().ok()?;
            }
            _ => return None,
        }
    }
    None
}

fn to_nibbles(data: &[u8]) -> Vec<u8> {
    let mut nibbles = Vec::with_capacity(data.len() * 2);
    for &b in data {
        nibbles.push(b >> 4);
        nibbles.push(b & 0x0F);
    }
    nibbles
}

fn decode_path(encoded: &[u8]) -> (bool, Vec<u8>) {
    let first = encoded[0];
    let is_leaf = (first & 0x20) != 0;
    let has_odd_len = (first & 0x10) != 0;
    
    let mut nibbles = Vec::new();
    if has_odd_len {
        nibbles.push(first & 0x0F);
    }
    for &b in &encoded[1..] {
        nibbles.push(b >> 4);
        nibbles.push(b & 0x0F);
    }
    (is_leaf, nibbles)
}
