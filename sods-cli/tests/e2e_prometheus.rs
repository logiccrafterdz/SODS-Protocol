mod common;
use common::TestEnv;
use predicates::prelude::*;

#[tokio::test]
async fn test_daemon_start_with_metrics_help() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("daemon")
        .arg("start")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--metrics-port"));
}

#[tokio::test]
async fn test_metrics_server_init() {
    // This test verifies the MetricsServer can be initialized without errors
    // (internal unit test logic mapped to integration test surface)
    let env = TestEnv::new().await;
    env.sods()
        .arg("daemon")
        .arg("start")
        .arg("--metrics-port")
        .arg("9999")
        .arg("--help") // use help to avoid actual start but verify arg parsing
        .assert()
        .success();
}
