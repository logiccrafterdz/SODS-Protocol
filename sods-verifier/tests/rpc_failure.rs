use sods_verifier::rpc::RpcClient;
use std::sync::Arc;
use tokio::time::Duration;

#[tokio::test]
async fn test_cache_behavior_on_rpc_failure() {
    // We'll use a bogus URL to trigger a failure
    let urls = vec!["http://localhost:1".to_string()]; // Nothing listening on port 1
    let client = RpcClient::new(&urls).expect("Failed to create RpcClient");
    
    let block_num = 7777777u64;
    client.clear_cache().await;
    
    println!("🧪 Testing failure behavior for block {}...", block_num);
    
    // First request should fail
    let res1 = client.fetch_logs_for_block(block_num).await;
    assert!(res1.is_err(), "First request should have failed with bogus URL");
    let initial_calls = client.get_fetch_count();
    println!("   First request failed as expected. Calls: {}", initial_calls);
    
    // Second request should also try to fetch (no negative caching)
    let res2 = client.fetch_logs_for_block(block_num).await;
    assert!(res2.is_err(), "Second request should also fail (not found in cache)");
    let final_calls = client.get_fetch_count();
    println!("   Second request failed as expected. Total calls: {}", final_calls);
    
    // Verify that at least one more call was attempted (or more due to retries)
    assert!(final_calls > initial_calls, "Subsequent request should have attempted another RPC call");
    
    println!("✅ Pass: RPC errors are not cached. System allows retrying failing blocks.");
}

#[tokio::test]
async fn test_concurrent_failure_convergence() {
    // 100 concurrent requests all failing
    let urls = vec!["http://localhost:1".to_string()];
    let client = Arc::new(RpcClient::new(&urls).unwrap());
    let block_num = 8888888u64;
    
    println!("🧪 Testing 100 concurrent failures...");
    
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let c = client.clone();
        tasks.push(tokio::spawn(async move {
            c.fetch_logs_for_block(block_num).await
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    for res in results {
        assert!(res.expect("Task panicked").is_err());
    }
    
    let total_calls = client.get_fetch_count();
    println!("📊 Total calls for 100 concurrent failures: {}", total_calls);
    
    // Since it's a connection error, it will likely try a few times and then fail.
    // The stampede prevention still applies to the *attempt* to fetch.
    // But since it never succeeds, it never enters the cache.
    // So one "stampede" happens, they all wait for the first write lock.
    // After the first one fails, the write lock is released.
    // Then the next one in line (the "double-check") will see it's NOT in cache and try AGAIN?
    // Wait, let's see the logic again.
}
