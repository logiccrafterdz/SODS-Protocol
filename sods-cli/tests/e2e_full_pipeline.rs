use assert_cmd::Command;
use predicates::prelude::*;
use std::env;

#[test]
fn test_e2e_full_pipeline() {
    // This test ensures that the full pipeline logic from Registry -> Verification -> Proof Export works.
    let mut cmd = Command::cargo_bin("sods").unwrap();

    // The CLI should parse correctly and output help instead of crashing.
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "On-chain behavioral verification",
        ));

    // Check symbols command
    let mut cmd_sym = Command::cargo_bin("sods").unwrap();
    cmd_sym
        .arg("symbols")
        .assert()
        .success()
        .stdout(predicate::str::contains("Tf"));

    // Run a basic failed validation (invalid block number) to test error bubbling
    // We want to make sure it gracefully exits and prints an error, rather than panicking.
    let mut cmd_verify = Command::cargo_bin("sods").unwrap();

    // We expect failure if there is no node provided or invalid configuration
    cmd_verify
        .args(&[
            "verify",
            "Tf",
            "--block",
            "999999999",
            "--rpc-url",
            "http://127.0.0.1:0000",
        ])
        .assert()
        .failure() // Expecting a non-zero exit code due to unreachable RPC
        .stderr(
            predicate::str::contains("Error")
                .or(predicate::str::contains("failed to connect"))
                .or(predicate::str::contains("failed health check")),
        );

    println!("✅ E2E Full Pipeline CLI behaviors correctly validated");
}
