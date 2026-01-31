use sods_verifier::rpc::RpcClient;
use std::sync::Arc;
use tokio::time::Duration;

/// Helper to get current memory usage on Windows (Working Set)
fn get_current_memory_usage() -> u64 {
    let pid = std::process::id();
    let output = std::process::Command::new("powershell")
        .args(&["-NoProfile", "-Command", &format!("(Get-Process -Id {}).WorkingSet64", pid)])
        .output();
    
    match output {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            s.parse::<u64>().unwrap_or(0)
        }
        Err(_) => 0,
    }
}

async fn run_concurrent_load(client: Arc<RpcClient>, count: usize, block_start: u64) {
    let mut handles = Vec::with_capacity(count);
    for i in 0..count {
        let c = client.clone();
        let b = block_start + (i % 5) as u64; // Rotate over 5 blocks to test cache hits/misses
        handles.push(tokio::spawn(async move {
            let _ = c.fetch_logs_for_block(b).await;
        }));
    }
    futures::future::join_all(handles).await;
}

#[tokio::test]
async fn test_no_memory_leak_under_sustained_load() {
    let urls = vec!["https://ethereum-sepolia.publicnode.com".to_string()];
    let client = Arc::new(RpcClient::new(&urls).unwrap());
    
    // Warm up
    run_concurrent_load(client.clone(), 100, 6000000).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let initial_memory = get_current_memory_usage();
    println!("📊 Initial memory usage: {} MB", initial_memory / (1024 * 1024));
    
    // 10 rounds of high concurrency
    for round in 1..=10 {
        run_concurrent_load(client.clone(), 200, 6000000 + (round * 10) as u64).await;
        // Minor sleep to allow GC/Cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        let current_mem = get_current_memory_usage();
        println!("   Round {}: {} MB", round, current_mem / (1024 * 1024));
    }
    
    let final_memory = get_current_memory_usage();
    let growth = if final_memory > initial_memory { final_memory - initial_memory } else { 0 };
    println!("🏁 Final memory usage: {} MB (Growth: {} MB)", 
        final_memory / (1024 * 1024), 
        growth / (1024 * 1024)
    );
    
    // We allow some growth for the LRU cache (100 blocks * ~100 logs each)
    // But it should be bounded and not linearly increasing with requests.
    // 20MB is a safe buffer for a 100-block LRU cache + tokio overhead.
    assert!(growth < 20 * 1024 * 1024, "Potential memory leak detected! Growth was {} bytes", growth);
    println!("✅ Pass: Memory usage remains stable under sustained load.");
}
