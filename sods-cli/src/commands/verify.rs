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
    
    // Choose verifier based on --no-header-proof flag
    let verifier: sods_verifier::BlockVerifier = if args.no_header_proof {
        match sods_verifier::BlockVerifier::new_rpc_only(rpc_url) {
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
                        verification_mode: "rpc_only".into(),
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
        }
    } else {
        match sods_verifier::BlockVerifier::new(rpc_url) {
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
                        verification_mode: "trustless".into(),
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
        
        // --- Causal Verification ---
        // 1. Build Causal Merkle Tree
        let cmt = sods_core::CausalMerkleTree::new(symbols.clone()); // CMT sorts by causality
        let root = cmt.root();
        
        // 2. Generate Proofs for matched sequence
        let mut proofs = Vec::new();
        let mut proof_generation_success = true;
        
        // Clone owned symbols for the proof struct
        let flow_symbols: Vec<sods_core::BehavioralSymbol> = matched_seq.iter().map(|&s| s.clone()).collect();
        
        for sym in &flow_symbols {
             if let Some(p) = cmt.generate_proof(sym.symbol(), sym.log_index()) {
                 proofs.push(p);
             } else {
                 proof_generation_success = false;
                 break;
             }
        }
        
        let causal_verified = if proof_generation_success {
            let causal_proof = sods_core::proof::CausalProof {
                root,
                symbols: flow_symbols.clone(),
                proofs,
            };
            causal_proof.verify(&root)
        } else {
            false
        };

         if args.json {
                let matched_seq_json: Vec<MatchedSymbol> = matched_seq.iter().map(|s| MatchedSymbol {
                    symbol: s.symbol.clone(),
                    log_index: s.log_index,
                }).collect();

                 let output = JsonOutput {
                    success: true,
                    symbol: args.symbol.clone(),
                    block: args.block,
                    chain: args.chain.clone(),
                    verified: causal_verified, // True only if causal check passes
                    occurrences: matched_seq.len(),
                    proof_size_bytes: flow_symbols.len() * 300, // Approx
                    time_ms: elapsed,
                    method: "causal_pattern".into(),
                    verification_mode: "rpc_only".into(), // Pattern verification uses RPC path
                    error: if causal_verified { None } else { Some("Causal verification failed".into()) },
                    matched_sequence: Some(matched_seq_json),
                };
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
         } else {
             if causal_verified {
                 println!("✅ Causal Pattern Verified!");
                 if let Some(first) = flow_symbols.first() {
                     println!("   Actor: {:?}", first.from);
                     print!("   Flow:  ");
                     for (i, sym) in flow_symbols.iter().enumerate() {
                         if i > 0 { print!(" -> "); }
                         if sym.is_from_deployer { print!("[Deployer] "); }
                         print!("{} (N:{})", sym.symbol(), sym.nonce);
                     }
                     println!();
                     println!("   Root:  0x{}", hex::encode(root));
                 }
             } else {
                  output::error("Pattern found but failed Causal Verification.");
                  output::hint("Events may not be contiguous or from same actor.");
             }
         }
         return if causal_verified { 0 } else { 1 };
    } else {
        if args.json {
             let output = JsonOutput {
                success: true,
                symbol: args.symbol.clone(),
                block: args.block,
                chain: args.chain.clone(),
                verified: false,
                occurrences: 0,
                proof_size_bytes: 0,
                time_ms: start.elapsed().as_millis() as u64,
                method: "pattern".into(),
                verification_mode: "rpc_only".into(),
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
