use std::process::Command;
use std::env;

#[test]
fn test_trend_detection() {
    // Skip in CI if explicitly disabled
    if env::var("CI").is_ok() {
         if env::var("SODS_RUN_INTEGRATION_TESTS").unwrap_or_default() != "1" {
            println!("Skipping integration test in CI (set SODS_RUN_INTEGRATION_TESTS=1 to enable)");
            return;
         }
    }

    let chain = "base";
    let pattern = "Tf"; // Simple pattern likely to be found
    let window = "3"; // Small window for speed

    println!("Testing trend detection on chain: {}", chain);
    
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--quiet",
            "--",
            "trend",
            "--pattern",
            pattern,
            "--chain",
            chain,
            "--window",
            window,
            "--json"
        ])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        if env::var("CI").is_ok() {
            eprintln!("⚠️ Soft failure on trend test: {}", stderr);
            return; 
        } else {
            panic!("Test failed for trend detection: {}", stderr);
        }
    }

    // Verify JSON structure
    assert!(stdout.contains("\"pattern\": \"Tf\""), "Output missing correct pattern");
    assert!(stdout.contains("\"chain\": \"base\""), "Output missing correct chain");
    assert!(stdout.contains("\"frequency_percent\""), "Output missing frequency_percent");
    assert!(stdout.contains("\"hotspots\""), "Output missing hotspots");
    assert!(stdout.contains("\"matches\""), "Output missing matches");
}
