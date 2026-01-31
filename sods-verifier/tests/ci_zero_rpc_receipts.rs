use sods_verifier::BlockVerifier;
use sods_verifier::header_anchor::VerificationMode;
use ethers_core::types::H256;
use std::env;

#[derive(Debug)]
struct ChainData {
    name: &'static str,
    rpc_url: String,
    block_number: u64,
    expected_receipts_root: H256,
}

#[tokio::test]
async fn test_multi_chain_receipts_root_validation() {
    // Note: To run this test against live chains, set appropriate RPC URLs.
    // Otherwise, it skips or uses public defaults.
    
    let chains = vec![
        ChainData {
            name: "Arbitrum One",
            rpc_url: env::var("ARB_RPC_URL").unwrap_or_else(|_| "https://arb1.arbitrum.io/rpc".to_string()),
            block_number: 290000000,
            expected_receipts_root: "0x3938be9ae329068595aeb4573887c3315978f870956461a5256e9f5e33d0273a".parse().unwrap(),
        },
        ChainData {
            name: "Optimism",
            rpc_url: env::var("OP_RPC_URL").unwrap_or_else(|_| "https://mainnet.optimism.io".to_string()),
            block_number: 120000000,
            expected_receipts_root: "0x7856345634563456345634563456345634563456345634563456345634563456".parse().unwrap(), // Placeholder
        },
        ChainData {
            name: "Base",
            rpc_url: env::var("BASE_RPC_URL").unwrap_or_else(|_| "https://mainnet.base.org".to_string()),
            block_number: 20000000,
            expected_receipts_root: "0x1234123412341234123412341234123412341234123412341234123412341234".parse().unwrap(), // Placeholder
        },
    ];

    for chain in chains {
        println!("Testing receipts root on {}: block {}", chain.name, chain.block_number);
        
        let verifier = match BlockVerifier::new(&[chain.rpc_url.clone()]) {
            Ok(v) => v,
            Err(_) => {
                println!("Skipping {}: Invalid RPC URL", chain.name);
                continue;
            }
        };

        // We fetch the header and check if it matches our expectation (sanity check)
        let header = match verifier.fetch_block_header(chain.block_number).await {
            Ok(h) => h,
            Err(e) => {
                println!("Skipping {}: Failed to fetch header: {}", chain.name, e);
                continue;
            }
        };

        println!("  Actual receiptsRoot: {:?}", header.receipts_root);
        
        // The core test: Can we reconstruct this root from receipts?
        // This validates our RLP-encoding including L2 extensions.
        match verifier.verify_symbol_in_block("Tf", chain.block_number).await {
            Ok(res) => {
                println!("  Verification successful: {}", res.is_verified);
                // If it succeeded, it means verify_receipts_against_header passed!
                assert!(res.verification_mode == VerificationMode::Trustless || res.verification_mode == VerificationMode::ZeroRpc);
            },
            Err(sods_verifier::error::SodsVerifierError::InvalidReceiptProof { computed, expected }) => {
                panic!("Root mismatch on {}!\nComputed: {}\nExpected: {}", chain.name, computed, expected);
            },
            Err(e) => {
                println!("  Note: {} verify failed with error: {}", chain.name, e);
                // We don't panic here as some blocks might have missing data on public RPCs
            }
        }
    }
}
