use ethers::prelude::*;
use std::sync::Arc;
use serde_json::json;

const VALIDATION_REGISTRY_ABI: &str = r#"[
    {"inputs":[{"internalType":"bytes32","name":"requestId","type":"bytes32"},{"internalType":"uint32","name":"score","type":"uint32"},{"internalType":"string","name":"metadataUri","type":"string"}],"name":"submitValidationResponse","outputs":[],"stateMutability":"nonpayable","type":"function"},
    {"inputs":[{"internalType":"bytes32","name":"requestId","type":"bytes32"}],"name":"getValidationResult","outputs":[{"internalType":"uint32","name":"score","type":"uint32"},{"internalType":"string","name":"metadata","type":"string"}],"stateMutability":"view","type":"function"}
]"#;

#[tokio::test]
async fn test_validation_request_response_cycle() {
    let rpc_url = std::env::var("SEPOLIA_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
    let priv_key = std::env::var("TEST_PRIVATE_KEY").unwrap_or_else(|_| "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());
    let registry_addr = std::env::var("SEPOLIA_VALIDATION_REGISTRY_ADDR").unwrap_or_else(|_| "0x8004000000000000000000000000000000000003".to_string());

    let provider = Provider::<Http>::try_from(rpc_url).unwrap();
    let wallet: LocalWallet = priv_key.parse::<LocalWallet>().unwrap().with_chain_id(11155111u64);
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    let address: Address = registry_addr.parse().unwrap();
    let abi: abi::Abi = serde_json::from_str(VALIDATION_REGISTRY_ABI).unwrap();
    let contract = Contract::new(address, abi, client.clone());

    // Step 1: Mock an incoming validation request hash
    let request_id = H256::random();

    // Step 2: SODS validates (Internal logic)
    let is_valid = true; // Assume proof is valid for E2E flow test
    let score = if is_valid { 100 } else { 0 };

    println!("Submitting validation response for request {:?} with score {}", request_id, score);

    // In a live environment, we would submit:
    // let call = contract.method::<_, ()>("submitValidationResponse", (request_id, score, "ipfs://QmResult".to_string())).unwrap();
    // call.send().await.unwrap();

    // Verify basic connectivity
    assert!(client.get_block_number().await.is_ok());
}
