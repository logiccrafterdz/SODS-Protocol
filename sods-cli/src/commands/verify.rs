//! Verify command implementation.

use clap::{Args, ValueEnum};
use serde::Serialize;

use crate::config::{get_chain, is_symbol_supported, SYMBOLS};
use crate::output;

/// Verification mode.
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum Mode {
    /// Try P2P first, fallback to RPC
    #[default]
    Auto,
    /// P2P only
    P2p,
    /// RPC only
    Rpc,
}

/// Arguments for the verify command.
#[derive(Args)]
pub struct VerifyArgs {
    /// Behavioral symbol to verify (e.g., Tf, Dep, Wdw)
    pub symbol: String,

    /// Block number to verify
    #[arg(short, long)]
    pub block: u64,

    /// Blockchain chain (sepolia, ethereum, base, arbitrum)
    #[arg(short, long, default_value = "sepolia")]
    pub chain: String,

    /// Verification mode
    #[arg(short, long, default_value = "auto")]
    pub mode: Mode,

    /// Custom RPC URL (overrides chain default)
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Timeout in seconds
    #[arg(short, long, default_value = "10")]
    pub timeout: u64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// JSON output structure.
#[derive(Serialize)]
struct JsonOutput {
    success: bool,
    symbol: String,
    block: u64,
    chain: String,
    verified: bool,
    occurrences: usize,
    proof_size_bytes: usize,
    time_ms: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    matched_sequence: Option<Vec<MatchedSymbol>>,
}

#[derive(Serialize)]
struct MatchedSymbol {
    symbol: String,
    log_index: u32,
}

/// Run the verify command.
pub async fn run(args: VerifyArgs) -> i32 {
    // 0. Check for Pattern
    if args.symbol.contains("->") || args.symbol.contains('→') || args.symbol.contains('{') {
        return run_pattern_verification(args).await;
    }

    // Validate symbol
    if !is_symbol_supported(&args.symbol) {
        if args.json {
            let output = JsonOutput {
                success: false,
                symbol: args.symbol.clone(),
                block: args.block,
                chain: args.chain.clone(),
                verified: false,
                occurrences: 0,
                proof_size_bytes: 0,
                time_ms: 0,
                method: "none".into(),
                error: Some(format!("Unsupported symbol: '{}'", args.symbol)),
                matched_sequence: None,
            };
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        } else {
            output::error(&format!("Symbol '{}' not supported.", args.symbol));
            output::hint(&format!(
                "Supported symbols: {}",
                SYMBOLS.iter().map(|(s, _)| *s).collect::<Vec<_>>().join(", ")
            ));
            output::info("Run `sods symbols` for full list with descriptions.");
        }
        return 1;
    }

    // Get chain config
    let chain_config = match get_chain(&args.chain) {
        Some(c) => c,
        None => {
            if args.json {
                let output = JsonOutput {
                    success: false,
                    symbol: args.symbol.clone(),
                    block: args.block,
                    chain: args.chain.clone(),
                    verified: false,
                    occurrences: 0,
                    proof_size_bytes: 0,
                    time_ms: 0,
                method: "none".into(),
                    error: Some(format!("Unknown chain: '{}'", args.chain)),
                    matched_sequence: None,
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                output::error(&format!("Chain '{}' not supported.", args.chain));
                output::info("Run `sods chains` for supported chains.");
            }
            return 1;
        }
    };

    // Determine RPC URL
    let rpc_url = args.rpc_url.as_deref().unwrap_or(chain_config.default_rpc);

    if !args.json {
        output::info(&format!(
            "Verifying '{}' in block {} ({})...",
            args.symbol, args.block, chain_config.description
        ));
    }

    // Create verifier and run
    let start = std::time::Instant::now();
    
    let verifier: sods_verifier::BlockVerifier = match sods_verifier::BlockVerifier::new(rpc_url) {
        Ok(v) => v,
        Err(e) => {
            if args.json {
                let output = JsonOutput {
                    success: false,
                    symbol: args.symbol.clone(),
                    block: args.block,
                    chain: args.chain.clone(),
                    verified: false,
                    occurrences: 0,
                    proof_size_bytes: 0,
                    time_ms: start.elapsed().as_millis() as u64,
                    method: "rpc".into(),
                    error: Some(format!("Failed to create verifier: {}", e)),
                    matched_sequence: None,
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                output::error(&format!("Failed to connect to RPC: {}", e));
                output::hint("Check your network connection or try a different RPC URL.");
            }
            return 1;
        }
    };

    match verifier.verify_symbol_in_block(&args.symbol, args.block).await {
        Ok(result) => {
            let elapsed = start.elapsed().as_millis() as u64;
            
            if args.json {
                let output = JsonOutput {
                    success: true,
                    symbol: args.symbol.clone(),
                    block: args.block,
                    chain: args.chain.clone(),
                    verified: result.is_verified,
                    occurrences: result.occurrences,
                    proof_size_bytes: result.proof_size_bytes,
                    time_ms: elapsed,
                    method: "rpc".into(),
                    error: result.error,
                    matched_sequence: None,
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                output::verification_result(
                    result.is_verified,
                    "RPC",
                    result.proof_size_bytes,
                    elapsed,
                    result.occurrences,
                );
                
                if !result.is_verified {
                    output::hint(&format!(
                        "Symbol '{}' may not exist in block {}.",
                        args.symbol, args.block
                    ));
                }
            }
            
            if result.is_verified { 0 } else { 1 }
        }
        Err(e) => {
            if args.json {
                let error_string: String = e.to_string();
                let output = JsonOutput {
                    success: false,
                    symbol: args.symbol.clone(),
                    block: args.block,
                    chain: args.chain.clone(),
                    verified: false,
                    occurrences: 0,
                    proof_size_bytes: 0,
                    time_ms: start.elapsed().as_millis() as u64,
                    method: "rpc".into(),
                    error: Some(error_string),
                    matched_sequence: None,
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                output::error(&format!("Verification failed: {}", e));
                output::hint("Try a different block number or check the chain.");
            }
            1
        }
    }
}

use sods_core::pattern::BehavioralPattern;

async fn run_pattern_verification(args: VerifyArgs) -> i32 {
    let start = std::time::Instant::now();

    // 1. Parse Pattern
    let pattern = match BehavioralPattern::parse(&args.symbol) {
        Ok(p) => p,
        Err(e) => {
             if args.json {
                // Return simple error json
                // We're reusing JsonOutput, though semantic match is loose
                let output = JsonOutput {
                    success: false,
                    symbol: args.symbol.clone(),
                    block: args.block,
                    chain: args.chain.clone(),
                    verified: false,
                    occurrences: 0,
                    proof_size_bytes: 0,
                    time_ms: 0,
                    method: "pattern".into(),
                    error: Some(format!("Invalid pattern: {}", e)),
                    matched_sequence: None,
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
             } else {
                 output::error(&format!("Invalid pattern: {}", e));
             }
             return 1;
        }
    };

    // 2. Resolve Chain
    let chain_config = match get_chain(&args.chain) {
        Some(c) => c,
        None => {
             if args.json {
                 // ... json error ...
                 println!("{{ \"error\": \"Unknown chain\" }}"); // simplify
             } else {
                 output::error(&format!("Chain '{}' not supported.", args.chain));
             }
             return 1;
        }
    };

    if !args.json {
        output::info(&format!("🔍 Verifying pattern '{}' in block {} ({})...", args.symbol, args.block, chain_config.description));
    }

    let verifier = match sods_verifier::BlockVerifier::new(chain_config.default_rpc) {
        Ok(v) => v,
        Err(e) => {
            if !args.json { output::error(&format!("Failed to connect to RPC: {}", e)); }
            return 1;
        }
    };

    // 3. Fetch all symbols in block
    let symbols = match verifier.fetch_block_symbols(args.block).await {
        Ok(s) => s,
        Err(e) => {
            if args.json {
                // ...
            } else {
                 output::error(&format!("Failed to fetch block symbols: {}", e));
            }
            return 1;
        }
    };

    // 4. Match Pattern
    if let Some(matched_seq) = pattern.matches(&symbols) {
        let elapsed = start.elapsed().as_millis() as u64;
         if args.json {
             // Extended JSON for pattern? reusing JsonOutput for now with verified=true
                let matched_seq_json: Vec<MatchedSymbol> = matched_seq.iter().map(|s| MatchedSymbol {
                    symbol: s.symbol.clone(),
                    log_index: s.log_index,
                }).collect();

                 let output = JsonOutput {
                    success: true,
                    symbol: args.symbol.clone(),
                    block: args.block,
                    chain: args.chain.clone(),
                    verified: true,
                    occurrences: matched_seq.len(),
                    proof_size_bytes: 0, // No specific proof for pattern yet (could aggregate)
                    time_ms: elapsed,
                    method: "pattern".into(),
                    error: None,
                    matched_sequence: Some(matched_seq_json),
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
         } else {
             println!("✅ Pattern matched in block {} ({})", args.block, chain_config.name);
             println!("   Matched sequence:");
             for sym in matched_seq {
                 println!("     {} @ idx={}", sym.symbol, sym.log_index);
             }
         }
         return 0;
    } else {
        if args.json {
             let output = JsonOutput {
                success: true, // Verification ran successfully, result is negative
                symbol: args.symbol.clone(),
                block: args.block,
                chain: args.chain.clone(),
                verified: false,
                occurrences: 0,
                proof_size_bytes: 0,
                time_ms: start.elapsed().as_millis() as u64,
                method: "pattern".into(),
                error: Some("Pattern not found".into()),
                matched_sequence: None,
            };
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        } else {
            output::error("Pattern not found in block.");
        }
        return 1;
    }
}
