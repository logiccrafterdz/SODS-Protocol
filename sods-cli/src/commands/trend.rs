//! Trend command implementation.

use clap::Args;
use colored::Colorize;
use serde::Serialize;
use std::time::Duration;
use tokio::time::sleep;

use crate::config::get_chain;
use crate::output;
use sods_core::pattern::BehavioralPattern;
use sods_verifier::BlockVerifier;

/// Arguments for the trend command.
#[derive(Args)]
pub struct TrendArgs {
    /// Behavioral pattern to detect (e.g., "LP+ -> Sw")
    #[arg(short, long)]
    pub pattern: String,

    /// Blockchain chain (sepolia, ethereum, base, arbitrum)
    #[arg(short, long, default_value = "sepolia")]
    pub chain: String,

    /// Number of recent blocks to scan (max 50)
    #[arg(short, long, default_value = "10")]
    pub window: u64,

    /// Custom RPC URL (overrides chain default)
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Timeout in seconds
    #[arg(long, default_value = "60")]
    pub timeout: u64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct TrendJsonOutput {
    pattern: String,
    chain: String,
    window: u64,
    frequency_percent: f64,
    matches: usize,
    hotspots: Vec<u64>,
    error: Option<String>,
}

pub async fn run(args: TrendArgs) -> i32 {
    // 1. Validate Window
    let window = if args.window > 50 {
        output::warn("Max window size is 50. Capping at 50.");
        50
    } else if args.window == 0 {
        output::error("Window size must be > 0.");
        return 1;
    } else {
        args.window
    };

    // 2. Parse Pattern
    let pattern = match BehavioralPattern::parse(&args.pattern) {
        Ok(p) => p,
        Err(e) => {
            if args.json {
                let output = TrendJsonOutput {
                    pattern: args.pattern.clone(),
                    chain: args.chain.clone(),
                    window,
                    frequency_percent: 0.0,
                    matches: 0,
                    hotspots: vec![],
                    error: Some(format!("Invalid pattern: {}", e)),
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                output::error(&format!("Invalid pattern: {}", e));
            }
            return 1;
        }
    };

    // 3. Resolve Chain/RPC
    let chain_config = match get_chain(&args.chain) {
        Some(c) => c,
        None => {
            if args.json {
                let output = TrendJsonOutput {
                    pattern: args.pattern.clone(),
                    chain: args.chain.clone(),
                    window,
                    frequency_percent: 0.0,
                    matches: 0,
                    hotspots: vec![],
                    error: Some(format!("Unknown chain: '{}'", args.chain)),
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                output::error(&format!("Chain '{}' not supported.", args.chain));
            }
            return 1;
        }
    };

    // 4. Initialize Verifier
    let rpc_urls: Vec<String> = if let Some(url) = args.rpc_url {
        vec![url]
    } else {
        chain_config.rpc_urls.iter().map(|s| s.to_string()).collect()
    };

    let is_l2 = chain_config.name != "ethereum" && chain_config.name != "sepolia";
    let profile = if is_l2 {
        sods_verifier::rpc::BackoffProfile::L2
    } else {
        sods_verifier::rpc::BackoffProfile::Ethereum
    };

    let verifier = match BlockVerifier::new(&rpc_urls) {
        Ok(v) => v.with_backoff_profile(profile),
        Err(e) => {
            if args.json {
                 println!("{{ \"error\": \"Failed to initialize RPCs: {}\" }}", e);
            } else {
                output::error(&format!("Failed to initialize RPCs: {}", e));
            }
            return 1;
        }
    };

    // Pre-flight health check
    if !verifier.health_check().await {
        if args.json {
            println!("{{ \"error\": \"All RPC endpoints failed health check.\" }}");
        } else {
            output::error("All primary and fallback RPC endpoints failed health check.");
        }
        return 1;
    }

    // 5. Get Head Block
    let head_block = match verifier.get_latest_block().await {
        Ok(b) => b,
        Err(e) => {
             if args.json {
                 println!("{{ \"error\": \"Failed to fetch latest block: {}\" }}", e);
            } else {
                output::error(&format!("Failed to fetch latest block: {}", e));
            }
            return 1;
        }
    };

    if !args.json {
        println!("   Head:   #{}", head_block);
        println!("{}", "Scanning...".yellow());
    }

    // 6. Scan Loop
    let mut hotspots = Vec::new();
    let start_block = head_block.saturating_sub(window).saturating_add(1); // e.g. head=100, win=10 -> 91..=100

    // Reverse order scan (newest first)
    for block_num in (start_block..=head_block).rev() {
        // Rate limit: 500ms
        sleep(Duration::from_millis(500)).await;

        // Fetch symbols
        let symbols = match verifier.fetch_block_symbols(block_num).await {
            Ok(s) => s,
            Err(e) => {
                if !args.json {
                    eprintln!("   ⚠️ Skipped block {}: {}", block_num, e);
                }
                continue;
            }
        };

        // Match pattern
        if pattern.matches(&symbols).is_some() {
            hotspots.push(block_num);
            if !args.json {
                print!("."); // Progress dot
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        } else {
             if !args.json {
                print!("_"); // No match dot
                use std::io::Write;
                std::io::stdout().flush().unwrap();
            }
        }
    }

    if !args.json {
        println!();
        println!();
    }

    // 7. Output Results
    let matches_count = hotspots.len();
    let frequency = if window > 0 {
        (matches_count as f64 / window as f64) * 100.0
    } else {
        0.0
    };

    if args.json {
        let output = TrendJsonOutput {
            pattern: args.pattern.clone(),
            chain: args.chain.clone(),
            window,
            frequency_percent: frequency,
            matches: matches_count,
            hotspots,
            error: None,
        };
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        let chart = if frequency > 50.0 { "🔥" } else if frequency > 20.0 { "📈" } else { "📉" };
        
        println!("{} Pattern \"{}\" on {} (last {} blocks):", chart, args.pattern.bold(), args.chain, window);
        println!("   Frequency: {}/{} blocks ({:.1}%)", matches_count, window, frequency);
        
        if matches_count > 0 {
            let hotspot_str = hotspots.iter().map(|b| format!("#{}", b)).collect::<Vec<_>>().join(", ");
            println!("   Hotspots:  {}", hotspot_str.green());
        } else {
             println!("   Hotspots:  None found");
        }
        println!();
    }

    0
}
