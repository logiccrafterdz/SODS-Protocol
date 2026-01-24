mod common;
use common::TestEnv;
use predicates::prelude::*;

#[tokio::test]
async fn test_listen_help() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("listen")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Listen for live behavioral alerts"));
}

#[tokio::test]
async fn test_daemon_start_with_websocket_help() {
    let env = TestEnv::new().await;
    env.sods()
        .arg("daemon")
        .arg("start")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--websocket-port"));
}

// Note: Full end-to-end WebSocket delivery test requires a running daemon.
// Since daemonizing is complex in tests (especially on Windows where it's a stub),
// we verify the CLI interface and arg parsing here.
// Core WebSocket logic is implemented with tokio-tungstenite and can be unit-tested
// within the daemon module if needed.
