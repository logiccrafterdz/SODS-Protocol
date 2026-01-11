//! Integration tests for sods-verifier.
//!
//! These tests require a valid INFURA_PROJECT_ID environment variable
//! to make real RPC calls to Sepolia.
//!
//! Run with: INFURA_PROJECT_ID=<key> cargo test --test integration -- --nocapture

use std::env;
use std::time::Duration;

use sods_verifier::BlockVerifier;

/// Get the RPC URL from environment, or skip test if not set.
fn get_rpc_url() -> Option<String> {
    let project_id = env::var("INFURA_PROJECT_ID").ok()?;
    
    if project_id.is_empty() || project_id == "your_project_id_here" {
        return None;
    }
    
    Some(format!("https://sepolia.infura.io/v3/{}", project_id))
}

/// Test block number from Python PoC.
const TEST_BLOCK: u64 = 10_002_322;

/// Maximum allowed verification time (2 seconds).
const MAX_VERIFICATION_TIME: Duration = Duration::from_secs(2);

#[tokio::test]
async fn test_verify_deposit_symbol() {
    let Some(rpc_url) = get_rpc_url() else {
        eprintln!("Skipping test: INFURA_PROJECT_ID not set");
        return;
    };

    let verifier = BlockVerifier::new(&rpc_url).expect("Failed to create verifier");
    
    let result = verifier
        .verify_symbol_in_block("Dep", TEST_BLOCK)
        .await
        .expect("Verification failed");

    println!("\n{}", result);
    println!("  RPC time: {:?}", result.rpc_fetch_time);
    println!("  Verify time: {:?}", result.verification_time);
    println!("  Total time: {:?}", result.total_time);

    // Assertions based on Python PoC results
    assert!(result.is_verified, "Dep should be verified in block {}", TEST_BLOCK);
    assert_eq!(result.occurrences, 2, "Expected 2 Dep occurrences");
    assert!(result.proof_size_bytes > 0, "Proof should have non-zero size");
    
    // Print BMT root for comparison with Python PoC
    if let Some(ref root) = result.merkle_root {
        println!("  BMT Root: 0x{}", hex::encode(root));
    }
}

#[tokio::test]
async fn test_verify_transfer_symbol() {
    let Some(rpc_url) = get_rpc_url() else {
        eprintln!("Skipping test: INFURA_PROJECT_ID not set");
        return;
    };

    let verifier = BlockVerifier::new(&rpc_url).expect("Failed to create verifier");
    
    let result = verifier
        .verify_symbol_in_block("Tf", TEST_BLOCK)
        .await
        .expect("Verification failed");

    println!("\n{}", result);

    // Assertions based on Python PoC results
    assert!(result.is_verified, "Tf should be verified in block {}", TEST_BLOCK);
    assert_eq!(result.occurrences, 20, "Expected 20 Tf occurrences");
}

#[tokio::test]
async fn test_verify_withdrawal_symbol() {
    let Some(rpc_url) = get_rpc_url() else {
        eprintln!("Skipping test: INFURA_PROJECT_ID not set");
        return;
    };

    let verifier = BlockVerifier::new(&rpc_url).expect("Failed to create verifier");
    
    let result = verifier
        .verify_symbol_in_block("Wdw", TEST_BLOCK)
        .await
        .expect("Verification failed");

    println!("\n{}", result);

    // Assertions based on Python PoC results
    assert!(result.is_verified, "Wdw should be verified in block {}", TEST_BLOCK);
    assert_eq!(result.occurrences, 1, "Expected 1 Wdw occurrence");
}

#[tokio::test]
async fn test_non_existent_symbol() {
    let Some(rpc_url) = get_rpc_url() else {
        eprintln!("Skipping test: INFURA_PROJECT_ID not set");
        return;
    };

    let verifier = BlockVerifier::new(&rpc_url).expect("Failed to create verifier");
    
    // LP+ likely doesn't exist in this block
    let result = verifier
        .verify_symbol_in_block("LP+", TEST_BLOCK)
        .await
        .expect("Verification should not fail for missing symbol");

    println!("\n{}", result);

    assert!(!result.is_verified, "LP+ should not be verified");
    assert_eq!(result.occurrences, 0, "Expected 0 occurrences");
    assert!(result.error.is_some(), "Should have error message");
}

#[tokio::test]
async fn test_unsupported_symbol() {
    let Some(rpc_url) = get_rpc_url() else {
        eprintln!("Skipping test: INFURA_PROJECT_ID not set");
        return;
    };

    let verifier = BlockVerifier::new(&rpc_url).expect("Failed to create verifier");
    
    let result = verifier
        .verify_symbol_in_block("InvalidSymbol", TEST_BLOCK)
        .await;

    assert!(result.is_err(), "Should fail for invalid symbol");
    
    let err = result.unwrap_err();
    println!("Error (expected): {}", err);
    assert!(err.to_string().contains("Unsupported symbol"));
}

#[tokio::test]
async fn test_performance_under_2_seconds() {
    let Some(rpc_url) = get_rpc_url() else {
        eprintln!("Skipping test: INFURA_PROJECT_ID not set");
        return;
    };

    let verifier = BlockVerifier::new(&rpc_url).expect("Failed to create verifier");
    
    let result = verifier
        .verify_symbol_in_block("Dep", TEST_BLOCK)
        .await
        .expect("Verification failed");

    println!("\nPerformance test:");
    println!("  Total time: {:?}", result.total_time);
    println!("  Max allowed: {:?}", MAX_VERIFICATION_TIME);

    assert!(
        result.total_time < MAX_VERIFICATION_TIME,
        "Verification took {:?}, exceeds limit of {:?}",
        result.total_time,
        MAX_VERIFICATION_TIME
    );
}
