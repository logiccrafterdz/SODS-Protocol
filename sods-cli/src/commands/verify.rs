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
    /// Cryptographically anchored verification (Bulk)
    Trustless,
    /// Fully trustless verification via Ethereum storage proofs (Zero-RPC)
    StorageProof,
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

    /// Reputation threshold for P2P consensus (0.0 - 1.0)
    #[arg(long, default_value = "0.1")]
    pub reputation_threshold: f32,

    /// Skip header-anchored verification (not recommended for production)
    #[arg(long)]
    pub no_header_proof: bool,
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
    verification_mode: String,
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
                verification_mode: "n/a".into(),
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
                    verification_mode: "n/a".into(),
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

    // Determine RPC URLs
    let rpc_urls: Vec<String> = if let Some(url) = args.rpc_url {
        vec![url]
    } else {
        chain_config.rpc_urls.iter().map(|s| s.to_string()).collect()
    };

    // Determine backoff profile
    let is_l2 = chain_config.name != "ethereum" && chain_config.name != "sepolia";
    let profile = if is_l2 {
        sods_verifier::rpc::BackoffProfile::L2
    } else {
        sods_verifier::rpc::BackoffProfile::Ethereum
    };

    if !args.json {
        output::info(&format!(
            "Verifying '{}' in block {} ({})...",
            args.symbol, args.block, chain_config.description
        ));
    }

    // Create verifier and run
    let start = std::time::Instant::now();
    
    // Determine if header proof (Trustless Mode) is required
    let require_header = match args.mode {
        Mode::Trustless => true,
        _ => !args.no_header_proof,
    };

    // Choose verifier based on requirements
    let verifier: sods_verifier::BlockVerifier = if !require_header {
        match sods_verifier::BlockVerifier::new_rpc_only(&rpc_urls) {
            Ok(v) => v.with_backoff_profile(profile),
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
                        verification_mode: "rpc_only".into(),
                        error: Some(format!("Failed to create verifier: {}", e)),
                        matched_sequence: None,
                    };
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                } else {
                    output::error(&format!("Failed to initialize RPCs: {}", e));
                    output::hint("Check your network connection or RPC endpoints.");
                }
                return 1;
            }
        }
    } else {
        match sods_verifier::BlockVerifier::new(&rpc_urls) {
            Ok(v) => v.with_backoff_profile(profile),
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
                        verification_mode: "trustless".into(),
                        error: Some(format!("Failed to create verifier: {}", e)),
                        matched_sequence: None,
                    };
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                } else {
                    output::error(&format!("Failed to initialize RPCs: {}", e));
                    output::hint("Check your network connection or RPC endpoints.");
                }
                return 1;
            }
        }
    };

    // Pre-flight health check
    if !verifier.health_check().await {
         if !args.json { 
             output::error("All primary and fallback RPC endpoints failed health check.");
         }
         // But we try to proceed anyway or fail early? 
         // For verify, we might want to try once more or fail.
         // Let's fail if all are dead.
         return 1;
    }

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
                    verification_mode: result.verification_mode.to_string(),
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
                    verification_mode: if args.no_header_proof { "rpc_only".into() } else { "trustless".into() },
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
                    verification_mode: "n/a".into(),
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

    let rpc_urls: Vec<String> = chain_config.rpc_urls.iter().map(|s| s.to_string()).collect();
    let is_l2 = chain_config.name != "ethereum" && chain_config.name != "sepolia";
    let profile = if is_l2 {
        sods_verifier::rpc::BackoffProfile::L2
    } else {
        sods_verifier::rpc::BackoffProfile::Ethereum
    };

    let verifier = match sods_verifier::BlockVerifier::new(&rpc_urls) {
        Ok(v) => v.with_backoff_profile(profile),
        Err(e) => {
            if !args.json { output::error(&format!("Failed to connect to RPC: {}", e)); }
            return 1;
        }
    };

    // 3. Verify Pattern using Optimized Pipeline (Filtering + Incremental BMT)
    match verifier.verify_pattern_in_block(&args.symbol, args.block).await {
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
                    method: "incremental_pattern".into(),
                    verification_mode: result.verification_mode.to_string(),
                    error: result.error,
                    matched_sequence: None, // Simplified for optimized path
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                if result.is_verified {
                    println!("✅ Pattern Verified (Optimized Path)!");
                    println!("   Occurrences: {}", result.occurrences);
                    println!("   Root:        0x{}", hex::encode(result.merkle_root.unwrap_or([0u8; 32])));
                    println!("   Time:        {} ms", elapsed);
                    println!("   Mode:        Incremental / Filtered");
                } else {
                    output::error("Pattern not found in block.");
                }
            }
            return if result.is_verified { 0 } else { 1 };
        }
        Err(e) => {
            if args.json {
                 // ... json error ...
            } else {
                 output::error(&format!("Pattern verification failed: {}", e));
            }
            return 1;
        }
    }
}
