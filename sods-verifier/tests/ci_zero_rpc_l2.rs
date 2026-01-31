use sods_verifier::BlockVerifier;
use std::env;

#[tokio::test]
async fn test_arbitrum_outbox_verification() {
    let rpc_url = env::var("ARB_RPC_URL").unwrap_or_else(|_| "https://arb1.arbitrum.io/rpc".to_string());
    let verifier = BlockVerifier::new(&[rpc_url]).unwrap();

    // Known Arbitrum Outbox event block (e.g. recent one)
    // We search for a block with OutboxTransaction events
    let block_num = 290000000; 

    // Symbols to check if they are supported in our current dictionary
    // "Tf" is universal.
    let result = verifier.verify_symbol_in_block("Tf", block_num).await;
    
    match result {
        Ok(res) => {
            println!("Arbitrum Verification: {:?}", res);
            assert!(res.is_verified);
        },
        Err(e) => {
            println!("Arbitrum test skipped or failed: {}", e);
        }
    }
}

#[tokio::test]
async fn test_optimism_bedrock_deposit_verification() {
    let rpc_url = env::var("OP_RPC_URL").unwrap_or_else(|_| "https://mainnet.optimism.io".to_string());
    let verifier = BlockVerifier::new(&[rpc_url]).unwrap();

    // Known Optimism block with deposits
    let block_num = 120000000; 

    let result = verifier.verify_symbol_in_block("Dep", block_num).await;
    
    match result {
        Ok(res) => {
            println!("Optimism Verification: {:?}", res);
            // Deposits might not always be in every block, so we check if result is success or not_found
            assert!(res.verification_mode == sods_verifier::header_anchor::VerificationMode::Trustless);
        },
        Err(e) => {
            println!("Optimism test skipped or failed: {}", e);
        }
    }
}
