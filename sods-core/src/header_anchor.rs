use ethers_core::types::{TransactionReceipt, H256};
use ethers_core::utils::rlp::RlpStream;
use ethers_core::utils::hex;
use sha3::{Digest, Keccak256};
use hash_db::Hasher;

/// Custom KeccakHasher for triehash
pub struct KeccakHasher;
impl Hasher for KeccakHasher {
    type Out = H256;
    type StdHasher = std::collections::hash_map::DefaultHasher;
    const LENGTH: usize = 32;
    fn hash(x: &[u8]) -> Self::Out {
        H256::from_slice(&Keccak256::digest(x))
    }
}

pub fn rlp_encode_receipt(receipt: &TransactionReceipt) -> Vec<u8> {
    // Determine the list size. Standard is 4.
    // L2s might append additional fields.
    let mut list_size = 4;
    
    // Check for L2 specific fields in `other` map that are part of the consensus commitment.
    // Optimism Bedrock: Deposit receipts (type 0x7E/126) include depositNonce and depositReceiptVersion.
    // L1 information fields (l1BlockNumber, l1Fee, etc.) are usually NOT part of the receiptsRoot.
    let l2_fields: Vec<&serde_json::Value> = [
        "depositNonce", 
        "depositReceiptVersion"
    ]
        .iter()
        .filter_map(|key| receipt.other.get(*key))
        .collect();
    
    list_size += l2_fields.len();

    let mut stream = RlpStream::new_list(list_size);

    // Status (1 = success, 0 = failure)
    let status = receipt.status.map(|s| s.as_u64()).unwrap_or(1);
    stream.append(&status);

    // Cumulative gas used
    stream.append(&receipt.cumulative_gas_used);

    // Logs bloom (256 bytes)
    stream.append(&receipt.logs_bloom.as_bytes().to_vec());

    // Logs
    stream.begin_list(receipt.logs.len());
    for log in &receipt.logs {
        stream.begin_list(3);
        stream.append(&log.address.as_bytes().to_vec());

        // Topics
        stream.begin_list(log.topics.len());
        for topic in &log.topics {
            stream.append(&topic.as_bytes().to_vec());
        }

        // Data
        stream.append(&log.data.to_vec());
    }

    // Append L2 fields if present
    for value in l2_fields {
        if let Some(n) = value.as_u64() {
            stream.append(&n);
        } else if let Some(s) = value.as_str() {
            if s.starts_with("0x") {
                let hex_str = if s.len() % 2 != 0 {
                    format!("0{}", &s[2..])
                } else {
                    s[2..].to_string()
                };
                let bytes = hex::decode(&hex_str).unwrap_or_default();
                // If the bytes represent a number, we might want to append as a number
                // but usually they are appended as raw bytes in these L2 extensions.
                stream.append(&bytes);
            } else {
                stream.append(&s);
            }
        } else if let Some(b) = value.as_bool() {
            stream.append(&b);
        } else if value.is_null() {
            stream.append_empty_data();
        } else if let Some(arr) = value.as_array() {
            stream.begin_list(arr.len());
            for item in arr {
                if let Some(n) = item.as_u64() {
                    stream.append(&n);
                } else if let Some(s) = item.as_str() {
                    if s.starts_with("0x") {
                        let hex_str = if s.len() % 2 != 0 {
                            format!("0{}", &s[2..])
                        } else {
                            s[2..].to_string()
                        };
                        let bytes = hex::decode(&hex_str).unwrap_or_default();
                        stream.append(&bytes);
                    } else {
                        stream.append(&s);
                    }
                } else if let Some(b) = item.as_bool() {
                    stream.append(&b);
                } else {
                    stream.append_empty_data();
                }
            }
        } else {
            stream.append_empty_data();
        }
    }

    let rlp_bytes = stream.out().to_vec();

    // Handle EIP-2718 typed transactions (Type 1, 2, 3, etc.)
    if let Some(t) = receipt.transaction_type {
        let t_u64 = t.as_u64();
        if t_u64 > 0 {
            let mut typed = vec![t_u64 as u8];
            typed.extend(rlp_bytes);
            return typed;
        }
    }
    
    rlp_bytes
}

/// Compute the Merkle-Patricia trie root from a list of receipts.
///
/// The receipt trie is ordered by transaction index (0, 1, 2, ...).
/// Keys are RLP-encoded indices, values are RLP-encoded receipts.
pub fn compute_receipts_root(receipts: &[TransactionReceipt]) -> H256 {
    if receipts.is_empty() {
        // Empty trie root
        return H256::from_slice(&Keccak256::digest([0x80]));
    }

    let encoded_receipts: Vec<Vec<u8>> = receipts.iter().map(rlp_encode_receipt).collect();
    
    // triehash::ordered_trie_root computes the root of a trie where keys are RLP-encoded indices
    triehash::ordered_trie_root::<KeccakHasher, _>(encoded_receipts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers_core::types::{Bloom, TransactionReceipt, H256, U64, U256};

    #[test]
    fn test_empty_receipts_root() {
        let receipts: Vec<TransactionReceipt> = vec![];
        let root = compute_receipts_root(&receipts);
        // keccak256(RLP("")) = keccak256(0x80)
        let expected = "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421";
        assert_eq!(format!("{:?}", root), expected);
    }

    #[test]
    fn test_single_receipt_root() {
        let mut receipt = TransactionReceipt::default();
        receipt.status = Some(U64::from(1));
        receipt.cumulative_gas_used = U256::from(21000);
        receipt.logs_bloom = Bloom::default();
        receipt.logs = vec![];
        
        let root = compute_receipts_root(&[receipt]);
        assert_ne!(root, H256::zero());
    }
}
