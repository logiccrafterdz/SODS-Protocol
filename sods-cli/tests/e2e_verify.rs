mod common;
use common::TestEnv;
use predicates::prelude::*;

#[tokio::test]
async fn test_verify_help() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("verify")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Behavioral symbol to verify"));
}

#[tokio::test]
async fn test_verify_invalid_chain() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("verify")
        .arg("Tf")
        .arg("--block")
        .arg("1000")
        .arg("--chain")
        .arg("nonexistent")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Chain 'nonexistent' not supported"));
}

#[tokio::test]
async fn test_verify_unsupported_symbol() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("verify")
        .arg("INVALID")
        .arg("--block")
        .arg("1000")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Symbol 'INVALID' not supported"));
}
