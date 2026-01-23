use clap::{Args, ValueEnum};
use crate::config::get_chain;
use crate::output;
use sods_core::pattern::BehavioralPattern;
use sods_core::BehavioralMerkleTree;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    Calldata,
    Json,
}

#[derive(Args)]
pub struct ExportProofArgs {
    /// Behavioral pattern to verify (e.g., "LP+ -> Sw")
    pub pattern: String,

    /// Block number
    #[arg(short, long)]
    pub block: u64,

    /// Blockchain chain
    #[arg(short, long, default_value = "sepolia")]
    pub chain: String,

    /// Output format
    #[arg(short, long, default_value = "calldata")]
    pub format: Format,

    /// Custom RPC URL
    #[arg(long)]
    pub rpc_url: Option<String>,
}

pub async fn run(args: ExportProofArgs) -> i32 {
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

    let verifier = match sods_verifier::BlockVerifier::new(&rpc_urls) {
        Ok(v) => v,
        Err(e) => {
            output::error(&format!("Failed to connect to RPC: {}", e));
            return 1;
        }
    };

    output::info(&format!("Fetching symbols for block {}...", args.block));
    let symbols = match verifier.fetch_block_symbols(args.block).await {
        Ok(s) => s,
        Err(e) => {
            output::error(&format!("Failed to fetch symbols: {}", e));
            return 1;
        }
    };

    let pattern = match BehavioralPattern::parse(&args.pattern) {
        Ok(p) => p,
        Err(e) => {
            output::error(&format!("Invalid pattern: {}", e));
            return 1;
        }
    };

    if let Some(matched) = pattern.matches(&symbols) {
        // Build Keccak BMT
        let bmt = BehavioralMerkleTree::new_keccak(symbols.clone());
        
        let chain_id = 11155111; // Default to Sepolia for now, should be in config
        
        if let Some(proof) = bmt.generate_onchain_proof(&matched, chain_id, args.block) {
            match args.format {
                Format::Calldata => {
                    let calldata = proof.to_calldata();
                    println!("0x{}", hex::encode(calldata));
                }
                Format::Json => {
                    println!("{}", serde_json::to_string_pretty(&proof).unwrap());
                }
            }
            0
        } else {
            output::error("Failed to generate on-chain proof.");
            1
        }
    } else {
        output::error("Pattern not found in block.");
        1
    }
}
