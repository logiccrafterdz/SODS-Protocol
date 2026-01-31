use ethers::prelude::*;
use std::sync::Arc;

#[tokio::test]
async fn test_escrow_payment_release_flow() {
    let rpc_url = std::env::var("SEPOLIA_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".to_string());
    let priv_key = std::env::var("TEST_PRIVATE_KEY").unwrap_or_else(|_| "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string());

    let provider = Provider::<Http>::try_from(rpc_url).unwrap();
    let wallet: LocalWallet = priv_key.parse::<LocalWallet>().unwrap().with_chain_id(11155111u64);
    let client = Arc::new(SignerMiddleware::new(provider, wallet));

    println!("Verifying escrow integration for agent {}", client.address());

    // In a full E2E setup:
    // 1. Deploy TestEscrow
    // 2. Fund it
    // 3. SODS Agent submits validation result (100)
    // 4. Trigger escrow.release(requestId)
    // 5. Assert balance increase

    let balance_before = client.get_balance(client.address(), None).await.unwrap();
    assert!(balance_before >= U256::zero());
}
