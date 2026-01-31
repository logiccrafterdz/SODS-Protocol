use sods_verifier::rpc::RpcClient;
use std::sync::Arc;
use tokio::time::Duration;
use std::sync::atomic::Ordering;

#[tokio::test]
async fn test_extreme_cache_stampede_prevention_1000() {
    // We use a real public RPC but we expect the cache to catch everything after the first call.
    // If the stampede prevention fails, the RPC provider might rate limit us (429), or the test will detect multiple calls.
    let urls = vec!["https://ethereum-sepolia.publicnode.com".to_string()];
    let client = RpcClient::new(&urls).expect("Failed to create RpcClient");
    let client = Arc::new(client);
    
    let block_num = 6000000u64;
    
    // Ensure cache is empty
    client.clear_cache().await;
    
    println!("🔥 Starting extreme stress test: 1000 concurrent tasks for block {}...", block_num);
    
    let mut tasks = Vec::with_capacity(1000);
    for i in 0..1000 {
        let c = client.clone();
        tasks.push(tokio::spawn(async move {
            // No sleep here, we want maximum "stampede" pressure
            c.fetch_logs_for_block(block_num).await
        }));
    }
    
    let start = std::time::Instant::now();
    let results = futures::future::join_all(tasks).await;
    let duration = start.elapsed();
    
    println!("⏱️ All 1000 requests completed in {:?}.", duration);
    
    // Verify all succeeded
    let mut logs_count = 0;
    for (i, res) in results.into_iter().enumerate() {
        let logs = res.expect("Task panicked")
            .unwrap_or_else(|e| panic!("Request {} failed: {}", i, e));
        
        if i == 0 {
            logs_count = logs.len();
        } else {
            assert_eq!(logs.len(), logs_count, "Inconsistent logs at index {}", i);
        }
    }
    
    // Check fetch count
    let total_calls = client.get_fetch_count();
    println!("📊 Total outgoing RPC calls: {}", total_calls);
    
    assert_eq!(total_calls, 1, "Cache stampede! Expected 1 RPC call, but got {}. Concurrency protection failed.", total_calls);
    println!("✅ Pass: 1000 requests handled with exactly 1 RPC call.");
}

#[tokio::test]
async fn test_sequential_vs_concurrent_performance() {
    let urls = vec!["https://ethereum-sepolia.publicnode.com".to_string()];
    let client = Arc::new(RpcClient::new(&urls).unwrap());
    let block_num = 6000001u64;
    
    // 1. Concurrent burst (Stampede)
    client.clear_cache().await;
    let start_c = std::time::Instant::now();
    let handles: Vec<_> = (0..50).map(|_| {
        let c = client.clone();
        tokio::spawn(async move { c.fetch_logs_for_block(block_num).await })
    }).collect();
    futures::future::join_all(handles).await;
    let duration_c = start_c.elapsed();
    let calls_c = client.get_fetch_count();
    
    // 2. Sequential (Hot cache)
    let start_s = std::time::Instant::now();
    for _ in 0..50 {
        client.fetch_logs_for_block(block_num).await.unwrap();
    }
    let duration_s = start_s.elapsed();
    
    println!("🚀 Concurrent burst (50 reqs): {:?} | Calls: {}", duration_c, calls_c);
    println!("🏃 Sequential (50 reqs):      {:?} | (Hot Cache)", duration_s);
    
    assert_eq!(calls_c, 1, "Concurrent burst should only make 1 call");
    assert!(duration_s < duration_c, "Sequential hot cache should be faster than initial burst");
}
