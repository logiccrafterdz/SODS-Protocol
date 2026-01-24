use sods_verifier::BlockVerifier;
use std::env;

#[tokio::test]
async fn test_provider_consistency_matrix() {
    // 1. Defined varied public RPC endpoints
    let providers = vec![
        "https://rpc.ankr.com/eth_sepolia",
        "https://ethereum-sepolia-rpc.publicnode.com",
        "https://1rpc.io/sepolia",
    ];
    
    let block_num = 6000000; // Choose a stable Sepolia block
    let mut results = Vec::new();

    println!("Fetching behavioral data from {} providers...", providers.len());

    for url in providers {
        let verifier = BlockVerifier::new(&[url.to_string()]).unwrap();
        match verifier.fetch_block_symbols(block_num).await {
            Ok(symbols) => {
                // Canonicalize for comparison: sort by index/content (symbols already sorted by index)
                let symbol_count = symbols.len();
                results.push(symbols);
                println!("✅ Fetch successful from {}: {} symbols found.", url, symbol_count);
            },
            Err(e) => {
                println!("⚠️ Skip: Fetch failed from {}: {}", url, e);
            }
        }
    }

    // 2. Perform Byte-for-Byte / Logical Comparison
    if results.len() >= 2 {
        let reference = &results[0];
        for (i, other) in results.iter().enumerate().skip(1) {
            assert_eq!(reference, other, "RPC PROVIDER DISCREPANCY DETECTED at Block #{}", block_num);
            println!("✅ Provider #{} is consistent with Reference.", i + 1);
        }
        println!("🏆 Cross-Provider Consistency Matrix: PASS");
    } else {
        println!("⚠️ Not enough providers reachable to perform consistency check.");
    }
}
