//! Discover command implementation.

use clap::Args;
use serde::Serialize;
use std::time::Duration;
use tokio::time::sleep;

use crate::config::{get_chain, is_symbol_supported, SYMBOLS};
use crate::output;

/// Arguments for the discover command.
#[derive(Args)]
pub struct DiscoverArgs {
    /// Behavioral symbol to scan for (e.g., Tf, Dep, Wdw)
    #[arg(short, long)]
    pub symbol: String,

    /// Blockchain chain (sepolia, ethereum, base, arbitrum)
    #[arg(short, long, default_value = "sepolia")]
    pub chain: String,

    /// Number of recent blocks to scan (max 200)
    #[arg(short, long, default_value = "50")]
    pub last: u64,

    /// Timeout in seconds
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// JSON output structure for discovery results.
#[derive(Serialize)]
struct DiscoveryOutput {
    success: bool,
    chain: String,
    symbol: String,
    scanned_blocks: u64,
    top_blocks: Vec<BlockCount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Serialize, Clone)]
struct BlockCount {
    block: u64,
    count: usize,
}

/// Run the discover command.
pub async fn run(args: DiscoverArgs) -> i32 {
    // 1. Validate inputs
    if !is_symbol_supported(&args.symbol) {
        if args.json {
            print_json_error(format!("Unsupported symbol: '{}'", args.symbol));
        } else {
            output::error(&format!("Symbol '{}' not supported.", args.symbol));
            output::hint(&format!(
                "Supported symbols: {}",
                SYMBOLS.iter().map(|(s, _)| *s).collect::<Vec<_>>().join(", ")
            ));
        }
        return 1;
    }

    let chain_config = match get_chain(&args.chain) {
        Some(c) => c,
        None => {
            if args.json {
                print_json_error(format!("Unknown chain: '{}'", args.chain));
            } else {
                output::error(&format!("Chain '{}' not supported.", args.chain));
                output::hint("Run `sods chains` for supported chains.");
            }
            return 1;
        }
    };

    let max_blocks = 200;
    let scan_count = if args.last > max_blocks {
        output::warn(&format!("Limiting scan to max {} blocks.", max_blocks));
        max_blocks
    } else {
        args.last
    };

    if !args.json {
        output::info(&format!(
            "🔍 Scanning last {} blocks on {} for '{}' events...",
            scan_count, chain_config.description, args.symbol
        ));
    }

    // 2. Initialize Verifier
    let verifier = match sods_verifier::BlockVerifier::new(chain_config.default_rpc) {
        Ok(v) => v,
        Err(e) => {
            if args.json {
                print_json_error(format!("Failed to create verifier: {}", e));
            } else {
                output::error(&format!("Failed to connect to RPC: {}", e));
            }
            return 1;
        }
    };

    // 3. Get latest block
    let latest_block = match verifier.get_latest_block().await {
        Ok(b) => b,
        Err(e) => {
            if args.json {
                print_json_error(format!("Failed to fetch latest block: {}", e));
            } else {
                output::error(&format!("Failed to fetch latest block: {}", e));
            }
            return 1;
        }
    };

    // 4. Scan blocks
    let mut results = Vec::new();
    let start_block = latest_block.saturating_sub(scan_count - 1);
    let end_block = latest_block;

    // Iterate in reverse (newest first)
    for block_num in (start_block..=end_block).rev() {
        match verifier.verify_symbol_in_block(&args.symbol, block_num).await {
            Ok(result) => {
                if result.occurrences > 0 {
                    results.push(BlockCount {
                        block: block_num,
                        count: result.occurrences,
                    });
                }
            }
            Err(e) => {
                // Log error but continue scanning
                if !args.json {
                    eprintln!("  ⚠️ Block {}: Scan failed ({})", block_num, e);
                }
            }
        }
        
        // Rate limiting delay
        sleep(Duration::from_millis(500)).await;
    }

    // 5. Rank results
    results.sort_by(|a, b| b.count.cmp(&a.count)); // Descending by count

    // 6. Output
    if args.json {
        let output = DiscoveryOutput {
            success: true,
            chain: args.chain.clone(),
            symbol: args.symbol.clone(),
            scanned_blocks: scan_count,
            top_blocks: results,
            error: None,
        };
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        if results.is_empty() {
             output::info("No events found in the scanned range.");
        } else {
            for (i, res) in results.iter().take(10).enumerate() {
                 let medal = match i {
                    0 => "🥇",
                    1 => "🥈",
                    2 => "🥉",
                    _ => "  ",
                };
                println!("{} Block {}: {} {} events", medal, res.block, res.count, args.symbol);
            }
        }
    }

    0
}

fn print_json_error(msg: String) {
    let output = DiscoveryOutput {
        success: false,
        chain: "unknown".into(),
        symbol: "unknown".into(),
        scanned_blocks: 0,
        top_blocks: vec![],
        error: Some(msg),
    };
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
