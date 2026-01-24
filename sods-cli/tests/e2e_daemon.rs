mod common;
use common::TestEnv;
use predicates::prelude::*;

#[tokio::test]
async fn test_daemon_help() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("daemon")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("System daemon management"));
}

#[tokio::test]
async fn test_daemon_start_help() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("daemon")
        .arg("start")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Start the daemon in background"));
}

#[tokio::test]
async fn test_daemon_invalid_subcommand() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("daemon")
        .arg("nonexistent")
        .assert()
        .failure();
}
