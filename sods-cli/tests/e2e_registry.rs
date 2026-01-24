mod common;
use common::TestEnv;
use predicates::prelude::*;

#[tokio::test]
async fn test_registry_lifecycle() {
    let env = TestEnv::new().await;
    
    // 1. Initial list
    env.sods()
        .arg("registry")
        .arg("list")
        .assert()
        .success();

    // 2. Add a contract
    env.sods()
        .arg("registry")
        .arg("add")
        .arg("--contract")
        .arg("0x1234567890123456789012345678901234567890")
        .arg("--deployer")
        .arg("0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef")
        .arg("--block")
        .arg("12345")
        .assert()
        .success();

    // 3. Verify it appears in the list
    env.sods()
        .arg("registry")
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("0x1234567890123456789012345678901234567890"));
}

#[tokio::test]
async fn test_registry_clear() {
    let env = TestEnv::new().await;
    
    // Clear
    env.sods()
        .arg("registry")
        .arg("clear")
        .assert()
        .success()
        .stdout(predicate::str::contains("Registry cleared"));
}
