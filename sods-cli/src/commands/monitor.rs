//! Monitor command implementation.

use clap::{Args, ValueEnum};
use colored::Colorize;
use std::time::Duration;
use tokio::time::sleep;

use crate::config::get_chain;
use crate::output;
use sods_core::pattern::BehavioralPattern;
use sods_verifier::{BlockVerifier, MempoolMonitor};

/// Monitoring mode.
#[derive(Debug, Clone, Copy, ValueEnum, Default, PartialEq)]
pub enum MonitorMode {
    /// Monitor finalized blocks (default)
    #[default]
    Block,
    /// Monitor pending transactions (mempool)
    Pending,
}

/// Arguments for the monitor command.
#[derive(Args)]
pub struct MonitorArgs {
    /// Behavioral pattern to monitor (e.g., "LP+ -> Sw")
    #[arg(short, long)]
    pub pattern: String,

    /// Blockchain chain (sepolia, ethereum, base, arbitrum)
    #[arg(short, long, default_value = "sepolia")]
    pub chain: String,

    /// Monitoring mode: block (default) or pending
    #[arg(short, long, default_value = "block")]
    pub mode: MonitorMode,

    /// Polling interval (e.g., "30s", "1m"). Min 10s. Used for block mode.
    #[arg(short, long, default_value = "30s")]
    pub interval: String,

    /// Custom RPC URL (overrides chain default)
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Enable predictive behavioral shadowing
    #[arg(long)]
    pub enable_shadows: bool,

    /// Automatically adapt polling interval based on RPC health
    #[arg(long)]
    pub auto_adapt: bool,
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
    // 1. Resolve Chain
    let chain_config = match get_chain(&args.chain) {
        Some(c) => c,
        None => {
            output::error(&format!("Chain '{}' not supported.", args.chain));
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

    output::header(&format!("🚨 Autonomous Monitor Active: {}", args.pattern));
    println!("   Chain:    {}", chain_config.description.cyan());
    println!("   Mode:     {:?}", args.mode);

    if args.mode == MonitorMode::Pending {
        return run_pending_monitor(args, chain_config, pattern).await;
    }

    // --- BLOCK MODE (Legacy) ---
    
    // 3. Parse Duration
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

    println!("   Interval: {}s", interval.as_secs());
    if args.auto_adapt {
        println!("   Adapt:    {}", "Enabled (Dynamic Throttling)".green());
    }
    println!("   Status:   Initializing...");

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

    let mut verifier = match BlockVerifier::new(&rpc_urls) {
        Ok(v) => v.with_backoff_profile(profile),
        Err(e) => {
            output::error(&format!("Failed to initialize RPCs: {}", e));
            return 1;
        }
    };

    // Pre-flight health check
    if !verifier.health_check().await {
        output::error("All primary and fallback RPC endpoints failed health check.");
        return 1;
    }

    // Load Plugins
    if let Ok(plugins) = crate::commands::symbols::load_local_plugins() {
        if !plugins.is_empty() {
            println!("   Plugins:  Loaded {} custom symbols", plugins.len());
            for p in plugins {
                verifier.register_plugin(p);
            }
        }
    }

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
    
    // Shadow State
    let mut active_shadows: Vec<sods_core::BehavioralShadow> = Vec::new();
    
    println!("{}", "Waiting for new blocks... (Ctrl+C to stop)".dimmed());

    // 6. Polling Loop
    loop {
        sleep(interval).await;
        
        // Auto-Adapt Logic
        if args.auto_adapt {
            let rpc_delay = verifier.current_rpc_delay();
            if rpc_delay > 500 { // If RPC is adding more than 500ms delay
                // Add proportional delay to the loop to let it cool off
                 sleep(Duration::from_millis(rpc_delay)).await;
            }
        }

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
                 println!("   ⚠️ Missed {} blocks. Skipping to head #{}", new_blocks_count, current_head);
                 last_scanned_block = current_head - 1; 
            }

            for block_num in (last_scanned_block + 1)..=current_head {
                if block_num > last_scanned_block + 1 {
                    sleep(Duration::from_millis(200)).await;
                }

                match verifier.fetch_block_symbols(block_num).await {
                    Ok(symbols) => {
                        // A. Check for Full Pattern Matches (Reactive)
                        if let Some(matched_seq) = pattern.matches(&symbols) {
                            let timestamp = chrono::Utc::now().to_rfc3339();
                            println!();
                            println!("🚨 {} Block #{} on {}", "PATTERN DETECTED!".red().bold(), block_num, args.chain);
                            println!("   Time:    {}", timestamp);
                            println!("   Pattern: {}", args.pattern.yellow());
                            println!("   Matched: {} events", matched_seq.len());
                            println!();
                        }

                        // B. Shadowing Logic (Proactive)
                        if args.enable_shadows {
                            // 1. Update Active Shadows
                            let mut resolved_indices = Vec::new();
                            let mut expired_indices = Vec::new();
                            
                            for (i, shadow) in active_shadows.iter_mut().enumerate() {
                                let status = shadow.check_block(block_num, &symbols);
                                match status {
                                    sods_core::shadow::ShadowStatus::Resolved => {
                                        println!("✅ Shadow Resolved: Actor {:?} completed pattern.", shadow.actor);
                                        resolved_indices.push(i);
                                    },
                                    sods_core::shadow::ShadowStatus::Deviation(reason) => {
                                        println!("⚠️  {} Deviation detected for actor {:?}", "PREDICTIVE ALERT:".magenta().bold(), shadow.actor);
                                        println!("    Reason: {}", reason);
                                        resolved_indices.push(i);
                                    },
                                    sods_core::shadow::ShadowStatus::Expired => {
                                        // Silent expiration or debug log
                                        expired_indices.push(i);
                                    },
                                    sods_core::shadow::ShadowStatus::Active => {
                                        // Still waiting
                                    }
                                }
                            }
                            
                            // Cleanup
                            let mut to_remove = [resolved_indices, expired_indices].concat();
                            to_remove.sort();
                            to_remove.dedup();
                            for i in to_remove.iter().rev() {
                                active_shadows.remove(*i);
                            }
                            
                            // 2. Spawn New Shadows
                            // Spawn if we see the FIRST step of the pattern
                            if let Some(first_step) = pattern.steps().first() {
                                if let sods_core::pattern::PatternStep::Exact(target, _) = first_step {
                                    for sym in &symbols {
                                        if sym.symbol == *target {
                                            if !active_shadows.iter().any(|s| s.actor == sym.from) {
                                                println!("🕵️  Spawning Shadow for actor {:?} (saw {})", sym.from, sym.symbol);
                                                let mut shadow = sods_core::BehavioralShadow::from_pattern(
                                                    &pattern, 
                                                    sym.from, 
                                                    sym.nonce,
                                                    block_num
                                                );
                                                shadow.current_step_index = 1; // Advance past first step
                                                active_shadows.push(shadow);
                                            }
                                        }
                                    }
                                }
                            }
                            
                            if !active_shadows.is_empty() {
                                println!("   Active Shadows: {}", active_shadows.len());
                            }
                        }
                    },
                    Err(e) => {
                         eprintln!("   ⚠️ Failed to scan block #{}: {}", block_num, e);
                    }
                }
            }
            last_scanned_block = current_head;
        }
    }
}

async fn run_pending_monitor(
    args: MonitorArgs, 
    chain_config: &crate::config::ChainConfig,
    pattern: BehavioralPattern
) -> i32 {
    // use futures_util::StreamExt; // Actually mempool monitor returns a Receiver

    // Resolve WS URL
    // If rpc_url is provided and starts with wss://, use it.
    // Else use chain defaults.
    let ws_url = if let Some(url) = &args.rpc_url {
        if url.starts_with("wss://") || url.starts_with("ws://") {
            url.as_str()
        } else {
             output::error("Custom RPC URL must be WebSocket (wss://) for pending mode.");
             return 1;
        }
    } else {
        match chain_config.default_ws {
            Some(url) => url,
            None => {
                output::error(&format!("WebSocket not supported for chain '{}'. Use --mode block or provide --rpc-url wss://...", args.chain));
                return 1;
            }
        }
    };

    println!("   URL:      {}", ws_url);
    println!("   Status:   Connecting to Mempool...");

    let monitor = match MempoolMonitor::connect(ws_url).await {
        Ok(m) => m,
        Err(e) => {
            output::error(&format!("Failed to connect to WebSocket: {}", e));
            return 1;
        }
    };

    let mut rx = match monitor.monitor(pattern, args.pattern.clone()).await {
        Ok(rx) => rx,
        Err(e) => {
            output::error(&format!("Failed to start monitor: {}", e));
            return 1;
        }
    };

    println!("{}", "Listening for pending transactions... (Ctrl+C to stop)".dimmed());

    while let Some(alert) = rx.recv().await {
        let timestamp = chrono::Utc::now().to_rfc3339();
        println!();
        println!("⚠️  {} Pending Tx on {}", "PENDING ALERT:".yellow().bold(), args.chain);
        println!("   Tx Hash:  {}", alert.tx_hash);
        println!("   Pattern:  {}", alert.pattern_name.cyan());
        println!("   Seq:      {}", alert.matched_sequence);
        println!("   Conf:     {:.0}%", alert.confidence * 100.0);
        println!("   Est. Inc: {}", alert.estimated_inclusion);
        println!("   Time:     {}", timestamp);
        println!();
    }

    0
}
