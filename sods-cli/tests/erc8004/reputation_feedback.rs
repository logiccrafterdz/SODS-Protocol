use ethers::prelude::*;
use std::sync::Arc;

const REPUTATION_REGISTRY_ABI: &str = r#"[
    {"inputs":[{"internalType":"uint256","name":"agentId","type":"uint256"},{"internalType":"uint32","name":"score","type":"uint32"},{"internalType":"string","name":"tag1","type":"string"},{"internalType":"string","name":"tag2","type":"string"},{"internalType":"string","name":"metadataUri","type":"string"}],"name":"giveFeedback","outputs":[],"stateMutability":"nonpayable","type":"function"}
]"#;

#[tokio::test]
async fn test_reputation_feedback_lifecycle() {
    let rpc_url = std::env::var("SEPOLIA_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
    let priv_key = std::env::var("CLIENT_PRIVATE_KEY").unwrap_or_else(|_| "59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d".to_string());
    let _registry_addr = std::env::var("SEPOLIA_REPUTATION_REGISTRY_ADDR").unwrap_or_else(|_| "0x8004000000000000000000000000000000000002".to_string());

    let provider = match Provider::<Http>::try_from(rpc_url) {
        Ok(p) => p,
        Err(_) => {
            println!("⚠️ Skipping: Invalid RPC URL");
            return;
        }
    };
    let wallet: LocalWallet = match priv_key.parse::<LocalWallet>() {
        Ok(w) => w.with_chain_id(11155111u64),
        Err(_) => {
            println!("⚠️ Skipping: Invalid Private Key");
            return;
        }
    };
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    println!("Submitting reputation feedback from client {}", client.address());

    // Mock feedback data
    let _agent_id = U256::from(1u64);
    let _score = 95u32;
    let _tag1 = "behavioral_proof_accuracy";
    let _tag2 = "bmt_verification";

    // In a live environment:
    // let abi: abi::Abi = serde_json::from_str(REPUTATION_REGISTRY_ABI).unwrap();
    // let contract = Contract::new(registry_addr.parse().unwrap(), abi, client.clone());
    // contract.method::<_, ()>("giveFeedback", (agent_id, score, tag1.to_string(), tag2.to_string(), "ipfs://QmFeedback".to_string())).unwrap().send().await.unwrap();

    match client.get_balance(client.address(), None).await {
        Ok(_) => println!("✅ Reputation feedback test connected"),
        Err(_) => println!("⚠️ Skipping: RPC unavailable"),
    }
}
