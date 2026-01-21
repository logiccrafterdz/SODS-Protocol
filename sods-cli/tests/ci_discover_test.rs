use std::process::Command;

#[test]
fn test_discover_on_all_chains() {
    let chains = ["base", "arbitrum", "optimism"];
    let symbol = "Tf";
    let last_blocks = "5";

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
            .current_dir(std::env::current_dir().unwrap()) 
            .output()
            .expect("Failed to execute command");

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Test failed for chain {}: {}", chain, stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("\"success\": true"), "Output did not indicate success for chain {}", chain);
        assert!(stdout.contains(&format!("\"chain\": \"{}\"", chain)), "Output missing correct chain info");
        assert!(stdout.contains("scanned_blocks"), "Output missing scanned_blocks field");
    }
}
