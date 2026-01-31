use sods_verifier::BlockVerifier;
use ethers_core::types::{H256, TransactionReceipt};
use ethers_core::utils::rlp::RlpStream;
use sha3::{Digest, Keccak256};
use std::env;
use hash_db::Hasher;

#[derive(Default, Debug, Clone, PartialEq)]
pub struct KeccakHasher;
impl Hasher for KeccakHasher {
    type Out = H256;
    type StdHasher = std::collections::hash_map::DefaultHasher;
    const LENGTH: usize = 32;
    fn hash(x: &[u8]) -> Self::Out {
        H256::from_slice(&Keccak256::digest(x))
    }
}

fn local_rlp_encode(receipt: &TransactionReceipt, l2_fields: &[&str]) -> Vec<u8> {
    let mut list_size = 4;
    let found_fields: Vec<(&str, &serde_json::Value)> = l2_fields.iter()
        .filter_map(|&k| receipt.other.get(k).map(|v| (k, v)))
        .collect();
    
    list_size += found_fields.len();
    let mut stream = RlpStream::new_list(list_size);
    
    stream.append(&receipt.status.map(|s| s.as_u64()).unwrap_or(1));
    stream.append(&receipt.cumulative_gas_used);
    stream.append(&receipt.logs_bloom.as_bytes().to_vec());
    
    stream.begin_list(receipt.logs.len());
    for log in &receipt.logs {
        stream.begin_list(3);
        stream.append(&log.address.as_bytes().to_vec());
        stream.begin_list(log.topics.len());
        for topic in &log.topics {
            stream.append(&topic.as_bytes().to_vec());
        }
        stream.append(&log.data.to_vec());
    }

    for (_, value) in found_fields {
        if let Some(n) = value.as_u64() {
            stream.append(&n);
        } else if let Some(s) = value.as_str() {
            let hex_str = if s.starts_with("0x") {
                if s.len() % 2 != 0 { format!("0{}", &s[2..]) } else { s[2..].to_string() }
            } else {
                s.to_string()
            };
            if let Ok(bytes) = hex::decode(&hex_str) {
                stream.append(&bytes);
            } else {
                stream.append(&s);
            }
        }
    }

    let rlp_bytes = stream.out().to_vec();
    if let Some(t) = receipt.transaction_type {
        let t_u64 = t.as_u64();
        if t_u64 > 0 && t_u64 < 0x80 {
            let mut typed = vec![t_u64 as u8];
            typed.extend(rlp_bytes);
            return typed;
        }
    }
    rlp_bytes
}

#[tokio::test]
async fn debug_l2_receipts_root() {
    let chains = vec![
        ("Arbitrum One", "https://arb1.arbitrum.io/rpc", 290000000),
        ("Optimism", "https://mainnet.optimism.io", 120000000),
    ];

    let field_perms = vec![
        vec!["l1BlockNumber", "gasUsedForL1"],
        vec!["gasUsedForL1", "l1BlockNumber"],
        vec!["l1BlockNumber"],
        vec!["depositNonce", "depositReceiptVersion"],
        vec!["depositReceiptVersion", "depositNonce"],
    ];

    for (name, default_rpc, block_num) in chains {
        let env_key = if name.contains("Arb") { "ARB_RPC_URL" } else { "OP_RPC_URL" };
        let rpc_url = env::var(env_key).unwrap_or_else(|_| default_rpc.to_string());
        let verifier = BlockVerifier::new(&[rpc_url]).unwrap();

        println!("--- Testing {} block {} ---", name, block_num);
        let header = verifier.rpc_client().fetch_block_header(block_num).await.unwrap();
        let receipts = verifier.rpc_client().fetch_block_receipts(block_num).await.unwrap();
        
        println!("  Header ReceiptsRoot: {:?}", header.receipts_root);

        for perm in &field_perms {
            let encoded: Vec<Vec<u8>> = receipts.iter().map(|r| local_rlp_encode(r, perm)).collect();
            let root = triehash::ordered_trie_root::<KeccakHasher, _>(encoded);
            if root == header.receipts_root {
                println!("  [MATCH!] Permutation {:?} produced the correct root!", perm);
            } else {
                println!("  [Fail] Perm {:?} -> {:?}", perm, root);
            }
        }
    }
}
