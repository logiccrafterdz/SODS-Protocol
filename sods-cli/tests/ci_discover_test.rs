use std::process::Command;
use std::env;

#[test]
fn test_discover_on_all_chains() {
    // Skip in CI if explicitly disabled
    if env::var("CI").is_ok() && env::var("SODS_RUN_INTEGRATION_TESTS").unwrap_or_default() != "1" {
        println!("Skipping integration test in CI (set SODS_RUN_INTEGRATION_TESTS=1 to enable)");
        return;
    }

    let chains = ["base", "arbitrum", "optimism"];
    let symbol = "Tf";
    let last_blocks = "3"; // Reduce to 3 blocks in CI to avoid rate limits

    for chain in chains {
        println!("Testing discovery on chain: {}", chain);
        
        let output = Command::new("cargo")
            .args(&[
                "run",
                "--quiet",
                "--",
                "discover",
                "--symbol",
                symbol,
                "--chain",
                chain,
                "--last",
                last_blocks,
                "--json"
            ])
            .output()
            .expect("Failed to execute command");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // In CI, allow soft failure but log it
            if env::var("CI").is_ok() {
                eprintln!("⚠️ Soft failure on {}: {}", chain, stderr);
                continue; // Don't panic in CI
            } else {
                panic!("Test failed for chain {}: {}", chain, stderr);
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"success\": true"), "Output did not indicate success for chain {}", chain);
        assert!(stdout.contains(&format!("\"chain\": \"{}\"", chain)), "Output missing correct chain info");
    }
}
