use sods_verifier::BlockVerifier;
use ethers_core::types::{TransactionReceipt, H256, Bloom};

#[tokio::test]
async fn test_trustless_mode_detects_tampering() {
    // Setup dummy receipts and header
    let mut receipt = TransactionReceipt::default();
    receipt.transaction_index = ethers_core::types::U64::from(0);
    receipt.status = Some(ethers_core::types::U64::from(1));
    receipt.cumulative_gas_used = ethers_core::types::U256::from(21000);
    
    let receipts = vec![receipt];
    let computed_root = sods_core::header_anchor::compute_receipts_root(&receipts);
    
    // Create a mismatching header
    let tampered_root = H256::random();
    
    // This is hard to test end-to-end without a mock RPC or complex setup.
    // We can verify the logic in header_anchor directly.
    
    let header = sods_verifier::header_anchor::BlockHeader {
        number: 1,
        hash: H256::random(),
        receipts_root: tampered_root,
        logs_bloom: Bloom::default(),
        parent_beacon_block_root: None,
        timestamp: 0,
    };
    
    let validation = sods_verifier::header_anchor::verify_receipts_against_header(&receipts, &header);
    assert!(!validation.is_valid, "Should detect root mismatch");
    assert_eq!(validation.computed_root, computed_root);
}
