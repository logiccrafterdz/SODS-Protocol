//! Monitor command implementation.

use clap::Args;
use colored::Colorize;
use std::time::Duration;
use tokio::time::sleep;

use crate::config::get_chain;
use crate::output;
use sods_core::pattern::BehavioralPattern;
use sods_verifier::BlockVerifier;

/// Arguments for the monitor command.
#[derive(Args)]
pub struct MonitorArgs {
    /// Behavioral pattern to monitor (e.g., "LP+ -> Sw")
    #[arg(short, long)]
    pub pattern: String,

    /// Blockchain chain (sepolia, ethereum, base, arbitrum)
    #[arg(short, long, default_value = "sepolia")]
    pub chain: String,

    /// Polling interval (e.g., "30s", "1m"). Min 10s.
    #[arg(short, long, default_value = "30s")]
    pub interval: String,

    /// Custom RPC URL (overrides chain default)
    #[arg(long)]
    pub rpc_url: Option<String>,
}

fn parse_duration(input: &str) -> Result<Duration, String> {
    let input = input.trim();
    if let Some(s) = input.strip_suffix('s') {
        let secs = s.parse::<u64>().map_err(|_| "Invalid seconds format")?;
        Ok(Duration::from_secs(secs))
    } else if let Some(m) = input.strip_suffix('m') {
        let mins = m.parse::<u64>().map_err(|_| "Invalid minutes format")?;
        Ok(Duration::from_secs(mins * 60))
    } else {
        // Default to seconds if valid number, else error
        let secs = input.parse::<u64>().map_err(|_| "Invalid duration format. Use '30s' or '1m'.")?;
        Ok(Duration::from_secs(secs))
    }
}

pub async fn run(args: MonitorArgs) -> i32 {
    // 1. Parse Duration
    let interval = match parse_duration(&args.interval) {
        Ok(d) => {
            if d.as_secs() < 10 {
                output::warn("Minimum interval is 10s. Adjusting to 10s.");
                Duration::from_secs(10)
            } else if d.as_secs() > 300 {
                output::warn("Maximum interval is 5m. Adjusting to 5m.");
                Duration::from_secs(300)
            } else {
                d
            }
        },
        Err(e) => {
            output::error(&format!("Invalid interval: {}", e));
            return 1;
        }
    };

    // 2. Parse Pattern
    let pattern = match BehavioralPattern::parse(&args.pattern) {
        Ok(p) => p,
        Err(e) => {
            output::error(&format!("Invalid pattern: {}", e));
            return 1;
        }
    };

    // 3. Resolve Chain/RPC
    let chain_config = match get_chain(&args.chain) {
        Some(c) => c,
        None => {
            output::error(&format!("Chain '{}' not supported.", args.chain));
            return 1;
        }
    };

    let rpc_url = args.rpc_url.as_deref().unwrap_or(chain_config.default_rpc);

    output::header(&format!("🚨 Autonomous Monitor Active: {}", args.pattern));
    println!("   Chain:    {}", chain_config.description.cyan());
    println!("   Interval: {}s", interval.as_secs());
    println!("   Status:   Initializing...");

    // 4. Initialize Verifier
    let verifier = match BlockVerifier::new(rpc_url) {
        Ok(v) => v,
        Err(e) => {
            output::error(&format!("Failed to connect to RPC: {}", e));
            return 1;
        }
    };

    // 5. Get Initial Block
    let mut last_scanned_block = match verifier.get_latest_block().await {
        Ok(b) => {
            println!("   Start:    Head Block #{}\n", b);
            b
        },
        Err(e) => {
            output::error(&format!("Failed to fetch initial block: {}", e));
            return 1;
        }
    };
    
    // Initial scan of current block to verify connection and maybe catch immediate action?
    // User expectation: "Start monitoring NOW". If logic is "scan > last_scanned", we miss the current block unless we set last_scanned = current - 1
    // Let's set last_scanned to current - 1 to scan the *current* head first?
    // Or just start from next new block. "Monitoring" usually implies future events.
    // Let's stick to future events to avoid noise, or maybe scan the current head.
    // Let's scan current head immediately just to confirm it works, then loop.
    // Actually, simple logic: last_scanned = current_head. Only scan *new* blocks (strictly >).
    println!("{}", "Waiting for new blocks... (Ctrl+C to stop)".dimmed());

    // 6. Polling Loop
    loop {
        sleep(interval).await;

        let current_head = match verifier.get_latest_block().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("   ⚠️ RPC Error: {}. Retrying in next interval...", e);
                continue;
            }
        };

        if current_head > last_scanned_block {
            let new_blocks_count = current_head - last_scanned_block;
            if new_blocks_count > 50 {
                 // Too many blocks (e.g. computer woke from sleep). Skip to latest to catch up.
                 println!("   ⚠️ Missed {} blocks. Skipping to head #{}", new_blocks_count, current_head);
                 last_scanned_block = current_head - 1; 
            }

            for block_num in (last_scanned_block + 1)..=current_head {
                // Rate limit inside burst scan: 200ms
                if block_num > last_scanned_block + 1 {
                    sleep(Duration::from_millis(200)).await;
                }

                match verifier.fetch_block_symbols(block_num).await {
                    Ok(symbols) => {
                        if let Some(matched_seq) = pattern.matches(&symbols) {
                            // ALERT!
                            let timestamp = chrono::Utc::now().to_rfc3339();
                            println!();
                            println!("🚨 {} Block #{} on {}", "PATTERN DETECTED!".red().bold(), block_num, args.chain);
                            println!("   Time:    {}", timestamp);
                            println!("   Pattern: {}", args.pattern.yellow());
                            println!("   Matched: {} events", matched_seq.len());
                            println!();
                        } else {
                            // Optional: print heartbeat or just stay silent?
                            // "Autonomous watchdog" -> silent until relevant.
                            // Maybe a small dot to show liveness?
                            // print!("."); use std::io::Write; std::io::stdout().flush().unwrap();
                        }
                    },
                    Err(e) => {
                         eprintln!("   ⚠️ Failed to scan block #{}: {}", block_num, e);
                    }
                }
            }
            last_scanned_block = current_head;
        }
        // Else: No new blocks, keep waiting
    }
}
