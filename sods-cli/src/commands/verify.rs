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
}

/// Run the verify command.
pub async fn run(args: VerifyArgs) -> i32 {
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
