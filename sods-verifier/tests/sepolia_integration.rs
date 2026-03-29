use sods_verifier::BlockVerifier;
use std::env;

#[tokio::test]
async fn test_sepolia_integration_latest_block() {
    let rpc_url = match env::var("SEPOLIA_RPC_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("Skipping Sepolia integration test: SEPOLIA_RPC_URL not set");
            return;
        }
    };

    let verifier = BlockVerifier::new(&[rpc_url]).expect("Failed to initialize verifier");

    // Test health check
    let health = verifier.health_check().await;
    assert!(
        health,
        "Health check should pass for a valid Sepolia RPC URL"
    );

    // Test verifying a common pattern (Tf) on a historical static block (e.g. 5000000)
    // Even if it fails to find the symbol, the execution itself should not error out.
    let result = verifier.verify_symbol_in_block("Tf", 5_000_000).await;

    // As long as the RPC is reachable and responsive, this should return Ok(VerificationResult)
    assert!(
        result.is_ok(),
        "Failed to query block 5000000: {:?}",
        result.err()
    );

    let verification = result.unwrap();
    println!("Historical Block Tf Present: {}", verification.is_verified);

    // Test querying an invalid block number (e.g., 999,999,999) to verify error handling
    let future_result = verifier.verify_symbol_in_block("Tf", 999_999_999).await;
    assert!(
        future_result.is_err(),
        "Querying an unreachable future block should return an Error"
    );
}
