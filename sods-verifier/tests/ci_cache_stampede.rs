use sods_verifier::rpc::RpcClient;
use std::sync::Arc;
use tokio::time::Duration;

#[tokio::test]
async fn test_concurrent_cache_stampede_prevention() {
    // Use a real but public RPC for the integration test
    let urls = vec!["https://ethereum-sepolia.publicnode.com".to_string()];
    let client = RpcClient::new(&urls).unwrap();
    let client = Arc::new(client);
    
    let block_num = 6000000u64; // A random stable block
    
    // Clear cache to ensure a fresh fetch
    client.clear_cache().await;
    
    println!("🚀 Spawning 100 concurrent requests for block {}...", block_num);
    
    let mut handles = Vec::new();
    for i in 0..100 {
        let c = client.clone();
        handles.push(tokio::spawn(async move {
            // Small variety in start times but mostly concurrent
            if i % 10 == 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            c.fetch_logs_for_block(block_num).await
        }));
    }
    
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await);
    }
    
    // Verify all 100 requests succeeded and returned same data
    let mut logs_len = None;
    for (i, res) in results.into_iter().enumerate() {
        let logs = res.expect("Task panicked")
            .unwrap_or_else(|e| panic!("Request {} failed: {}", i, e));
        
        if let Some(len) = logs_len {
            assert_eq!(logs.len(), len, "Inconsistent logs length at index {}", i);
        } else {
            logs_len = Some(logs.len());
        }
    }
    
    // THE CRITICAL CHECK: fetch_count should be exactly 1
    let fetch_count = client.get_fetch_count();
    println!("📊 Total RPC calls made: {}", fetch_count);
    
    assert_eq!(fetch_count, 1, "Cache stampede detected! Expected 1 RPC call, got {}", fetch_count);
    println!("✅ Cache stampede prevention verified: Only 1 RPC call for 100 concurrent requests.");
}
