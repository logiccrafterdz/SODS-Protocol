use sods_verifier::BlockVerifier;
use sods_verifier::rpc::BackoffProfile;

#[tokio::test]
async fn test_rpc_failover_logic() {
    // 1. One dead URL, one live URL (Sepolia public)
    let urls = vec![
        "https://invalid-dead-rpc-url.com".to_string(),
        "https://ethereum-sepolia.publicnode.com".to_string(),
    ];

    let verifier = BlockVerifier::new(&urls).unwrap();
    
    // This should succeed by skipping the first URL
    let result = verifier.get_latest_block().await;
    assert!(result.is_ok(), "Should failover to the second working RPC");
    println!("✅ Failover to secondary RPC successful");
}

#[tokio::test]
async fn test_backoff_profiles_delays() {
    let eth = BackoffProfile::Ethereum;
    let l2 = BackoffProfile::L2;
    
    assert!(l2.delays()[0] > eth.delays()[0], "L2 should have longer initial delay");
    assert!(l2.delays()[2] > eth.delays()[2], "L2 should have longer max delay");
}

#[tokio::test]
async fn test_total_exhaustion_error() {
    let urls = vec![
        "https://dead-1.com".to_string(),
        "https://dead-2.com".to_string(),
    ];

    let verifier = BlockVerifier::new(&urls).unwrap();
    let result = verifier.get_latest_block().await;
    
    assert!(result.is_err(), "Should return error when all RPCs are dead");
}
