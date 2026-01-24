mod common;
use common::TestEnv;
use predicates::prelude::*;

#[tokio::test]
async fn test_export_proof_help() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("export-proof")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Export an on-chain verifiable behavioral proof"));
}

#[tokio::test]
async fn test_export_proof_validation() {
    let env = TestEnv::new().await;
    
    // Test that missing arguments fail gracefully (block is required)
    env.sods()
        .arg("export-proof")
        .arg("Tf")
        .assert()
        .failure()
        .stderr(predicate::str::contains("the following required arguments were not provided"));
}

// Full export-proof mocking would require many RpcClient responses (header, logs, etc.)
// For now, we'll verify the CLI command availability.
// Actual proof generation logic is tested in sods-core and sods-verifier.
