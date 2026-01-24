use ethers_core::types::{TransactionReceipt, H256};
use ethers_core::utils::rlp::RlpStream;
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

/// RLP-encode a receipt for trie inclusion.
///
/// Receipts are encoded as:
/// - Legacy: RLP([status, cumulative_gas, logs_bloom, logs])
/// - EIP-2718: type_byte || RLP([status, cumulative_gas, logs_bloom, logs])
///
/// Note: Arbitrum and Optimism may have additional fields at the end of the list.
pub fn rlp_encode_receipt(receipt: &TransactionReceipt) -> Vec<u8> {
    // Determine the list size. Standard is 4.
    // L2s might append additional fields.
    // For Optimism Bedrock: some receipts are standard, others might have depositNonce.
    // However, `ethers-core`'s TransactionReceipt doesn't expose L2 fields directly.
    // We use `receipt.other` if available or assume standard for now but prepare the structure.
    
    let list_size = 4;
    
    // Check for L2 specific fields in `other` map (if supported by the provider)
    let has_l2_fields = !receipt.other.is_empty();
    if has_l2_fields {
        // This is a heuristic: if there are extra fields, we might need a longer RLP list.
        // For Optimism, we often see 4 fields. For Arbitrum, it varies by version.
    }

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

    // --- L2 Specific Extensions ---
    // In a full implementation, we would check the chain_id or receipt metadata
    // to append fields like `depositNonce` (Optimism) or `cumulativeGasUsed` variants (Arbitrum).
    // For v1.2, we stick to the 4 standard fields but allow extension points.

    let rlp_bytes = stream.out().to_vec();

    // Handle EIP-2718 typed transactions (Type 1, 2, 3, etc.)
    if let Some(t) = receipt.transaction_type {
        let t_u64 = t.as_u64();
        if t_u64 > 0 {
            // Type 0x7E is Optimism Deposit
            // Type 0x64 is Arbitrum
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
        // Empty trie root = keccak256(RLP(""))
        return H256::from_slice(&Keccak256::digest(&[0x80]));
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
