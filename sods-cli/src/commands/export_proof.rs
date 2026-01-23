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

    /// Include beacon root anchor for on-chain verification
    #[arg(long)]
    pub anchored: bool,

    /// Private key to sign the behavioral commitment (hex)
    #[arg(long)]
    pub signing_key: Option<String>,

    /// Address authorized to sign commitments (hex)
    #[arg(long)]
    pub trusted_signer: Option<String>,
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
        
        let chain_id = 11155111; // Default to Sepolia
        
        // Fetch beacon root and timestamp if anchored
        let (beacon_root, timestamp, receipts_root) = if args.anchored {
            output::info("Fetching block metadata for anchoring...");
            match verifier.fetch_block_header(args.block).await {
                Ok(header) => (header.parent_beacon_block_root.map(|h| h.0), header.timestamp, Some(header.receipts_root.0)),
                Err(e) => {
                    output::error(&format!("Failed to fetch block header for anchoring: {}", e));
                    return 1;
                }
            }
        } else {
            (None, 0, None)
        };

        let mut proof = match bmt.generate_onchain_proof(&matched, chain_id, args.block, beacon_root, timestamp) {
            Some(p) => p,
            None => {
                output::error("Failed to generate on-chain proof.");
                return 1;
            }
        };

        proof.receipts_root = receipts_root;

        // Signing logic
        if let Some(key_str) = args.signing_key {
            output::info("Signing behavioral commitment...");
            use ethers_signers::{LocalWallet, Signer};
            
            let wallet: LocalWallet = match key_str.parse() {
                Ok(w) => w,
                Err(e) => {
                    output::error(&format!("Invalid signing key: {}", e));
                    return 1;
                }
            };

            let commitment = sods_core::BehavioralCommitment::new(
                chain_id,
                args.block,
                receipts_root.unwrap_or([0u8; 32]),
                bmt.root(),
            );

            let hash = commitment.hash();
            
            // Sign the commitment hash
            match wallet.sign_message(hash).await {
                Ok(sig) => {
                    proof.signature = Some(sig.to_vec());
                    output::success(&format!("Commitment signed by {}", wallet.address()));
                }
                Err(e) => {
                    output::error(&format!("Failed to sign commitment: {}", e));
                    return 1;
                }
            }
        }

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
        output::error("Pattern not found in block.");
        1
    }
}
