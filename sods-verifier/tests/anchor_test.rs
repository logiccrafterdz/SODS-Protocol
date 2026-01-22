//! Integration tests for Block Header Anchored Verification.
//!
//! Requires INFURA_PROJECT_ID to run.

use sods_verifier::{BlockVerifier, VerificationMode};
use std::env;

fn get_rpc_url() -> Option<String> {
    let project_id = env::var("INFURA_PROJECT_ID").ok()?;
    if project_id.is_empty() { return None; }
    Some(format!("https://sepolia.infura.io/v3/{}", project_id))
}

const TEST_BLOCK: u64 = 10_002_322;

#[tokio::test]
async fn test_trustless_verification_success() {
    let Some(rpc_url) = get_rpc_url() else {
        println!("Skipping test: INFURA_PROJECT_ID not set");
        return;
    };

    // Default is trustless (require_header_proof = true)
    let verifier = BlockVerifier::new(&rpc_url).unwrap();

    let result = verifier
        .verify_symbol_in_block("Tf", TEST_BLOCK)
        .await
        .expect("Verification failed");

    assert_eq!(result.verification_mode, VerificationMode::Trustless);
    assert_eq!(result.occurrences, 20); // Known value for this block
    assert!(result.is_verified);
    
    println!("✅ Trustless verification passed: {:?}", result);
}

#[tokio::test]
async fn test_rpc_only_mode_success() {
    let Some(rpc_url) = get_rpc_url() else {
        println!("Skipping test: INFURA_PROJECT_ID not set");
        return;
    };

    // Explicitly disable header check
    let verifier = BlockVerifier::new_rpc_only(&rpc_url).unwrap();

    let result = verifier
        .verify_symbol_in_block("Tf", TEST_BLOCK)
        .await
        .expect("Verification failed");

    assert_eq!(result.verification_mode, VerificationMode::RpcOnly);
    assert!(result.is_verified);

    println!("✅ RPC-Only verification passed: {:?}", result);
}
