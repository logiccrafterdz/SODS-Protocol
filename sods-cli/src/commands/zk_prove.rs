use clap::Args;
use serde::Serialize;
use std::fs::File;
use std::io::Write;

use crate::config::get_chain;
use crate::output;
use sods_verifier::BlockVerifier;

#[derive(Args)]
pub struct ZkProveArgs {
    /// Behavioral pattern to prove (e.g., "LP+ -> Sw -> LP-")
    pub pattern: String,

    /// Block number to verify and prove
    #[arg(short, long)]
    pub block: u64,

    /// Blockchain chain (sepolia, ethereum, base, arbitrum)
    #[arg(short, long, default_value = "sepolia")]
    pub chain: String,

    /// Custom RPC URL (overrides chain default)
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Serialize)]
struct ZkProofOutput {
    success: bool,
    pattern: String,
    block: u64,
    chain: String,
    valid: bool,
    receipt_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn run(args: ZkProveArgs) -> i32 {
    let chain_config = match get_chain(&args.chain) {
        Some(c) => c,
        None => {
            output::error(&format!("Chain '{}' not supported.", args.chain));
            return 1;
        }
    };

    let rpc_urls: Vec<String> = if let Some(url) = args.rpc_url {
        vec![url]
    } else {
        chain_config.rpc_urls.iter().map(|s| s.to_string()).collect()
    };

    if !args.json {
        output::info(&format!(
            "Generating ZK proof for pattern '{}' in block {} ({})...",
            args.pattern, args.block, chain_config.description
        ));
        output::warning("This may take a few minutes as it runs in the zkVM...");
    }

    let start = std::time::Instant::now();
    
    // 1. Create verifier and fetch symbols
    let verifier = match BlockVerifier::new(&rpc_urls) {
        Ok(v) => v,
        Err(e) => {
            output::error(&format!("Failed to create verifier: {}", e));
            return 1;
        }
    };

    let symbols = match verifier.fetch_block_symbols(args.block).await {
        Ok(s) => s,
        Err(e) => {
            output::error(&format!("Failed to fetch symbols: {}", e));
            return 1;
        }
    };

    // 2. Generate ZK Proof
    let chain_id = chain_config.chain_id;
    let receipt = match sods_zk::prove_behavior(symbols, &args.pattern, args.block, chain_id) {
        Ok(r) => r,
        Err(e) => {
            output::error(&format!("ZK Prover failed: {}", e));
            return 1;
        }
    };

    // 3. Extract metadata from receipt journal
    // Tuple: (blockNumber, chainId, pattern, result)
    let journal_data: (u64, u64, String, bool) = match receipt.journal.decode() {
        Ok(v) => v,
        Err(e) => {
            output::error(&format!("Failed to decode receipt journal: {}", e));
            return 1;
        }
    };
    let valid = journal_data.3;

    // 4. Save artifacts
    let receipt_path = "proof.bin";
    let journal_path = "journal.bin";
    let receipt_bytes = bincode::serialize(&receipt).unwrap_or_default();
    
    if let Ok(mut file) = File::create(receipt_path) {
        let _ = file.write_all(&receipt_bytes);
    }
    
    if let Ok(mut file) = File::create(journal_path) {
        let _ = file.write_all(&receipt.journal.bytes);
    }

    let result = ZkProofOutput {
        success: true,
        pattern: args.pattern.clone(),
        block: args.block,
        chain: args.chain.clone(),
        valid,
        receipt_file: receipt_path.to_string(),
        error: None,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        output::success("ZK Proof generated successfully!");
        println!("Validity: {}", if valid { "MATCH" } else { "NO MATCH" });
        println!("Receipt: {}", receipt_path);
        println!("Journal: {} (use for on-chain verification)", journal_path);
        println!("Time taken: {:?}", start.elapsed());
        
        let public_json = "public.json";
        if let Ok(mut file) = File::create(public_json) {
            let _ = file.write_all(serde_json::to_string_pretty(&result).unwrap().as_bytes());
        }
    }

    0
}
