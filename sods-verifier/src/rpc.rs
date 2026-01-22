//! RPC client for fetching blockchain data.
//!
//! Wraps `ethers_providers::Provider` with LRU caching,
//! exponential backoff retry logic, and rate limit handling.

use ethers_core::types::{BlockNumber, Filter, Log};
use ethers_providers::{Http, Middleware, Provider};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use crate::error::{Result, SodsVerifierError};

/// Default cache size (number of blocks).
const DEFAULT_CACHE_SIZE: usize = 100;

/// Base backoff delays in milliseconds (adaptive).
const MIN_ADAPTIVE_DELAY_MS: u64 = 100;
const MAX_ADAPTIVE_DELAY_MS: u64 = 5000;

/// Jitter percentage for backoff (±10%).
const JITTER_PERCENT: f64 = 0.1;

#[derive(Clone)]
pub struct RpcClient {
    provider: Arc<Provider<Http>>,
    cache: Arc<Mutex<LruCache<u64, Vec<Log>>>>,
    /// Adaptive delay in milliseconds to be added to requests
    adaptive_delay: Arc<std::sync::atomic::AtomicU64>,
    #[cfg(test)]
    pub(crate) fetch_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl RpcClient {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| SodsVerifierError::RpcError(format!("Invalid RPC URL: {}", e)))?;

        let cache_size = std::env::var("SODS_RPC_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_CACHE_SIZE);

        let cache = LruCache::new(
            NonZeroUsize::new(cache_size).unwrap_or(NonZeroUsize::new(1).unwrap()),
        );

        Ok(Self {
            provider: Arc::new(provider),
            cache: Arc::new(Mutex::new(cache)),
            adaptive_delay: Arc::new(std::sync::atomic::AtomicU64::new(MIN_ADAPTIVE_DELAY_MS)),
            #[cfg(test)]
            fetch_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    /// Update adaptive delay based on request outcome.
    fn update_adaptive_delay(&self, success: bool, error_kind: Option<&str>) {
        let current = self.adaptive_delay.load(std::sync::atomic::Ordering::Relaxed);
        
        let new_delay = if success {
            // Decay: Reduce delay by 10% on success, down to Min
            std::cmp::max(MIN_ADAPTIVE_DELAY_MS, (current as f64 * 0.9) as u64)
        } else {
            // Increase: specific errors trigger faster backoff
            if let Some(kind) = error_kind {
                if kind.contains("rate limit") || kind.contains("too many requests") {
                    // Double delay on rate limit
                     std::cmp::min(MAX_ADAPTIVE_DELAY_MS, current * 2)
                } else if kind.contains("timeout") {
                     std::cmp::min(MAX_ADAPTIVE_DELAY_MS, (current as f64 * 1.5) as u64)
                } else {
                     current // Other errors might not need throttling
                }
            } else {
                current 
            }
        };

        if new_delay != current {
            if new_delay > current {
                // Only log when increasing significantly to avoid noise
                 // eprintln!("Backing off RPC: {}ms", new_delay); 
            }
            self.adaptive_delay.store(new_delay, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub async fn get_latest_block(&self) -> Result<u64> {
        let result = self.provider.get_block_number().await;
        
        match result {
             Ok(n) => {
                 self.update_adaptive_delay(true, None);
                 Ok(n.as_u64())
             },
             Err(e) => {
                 self.update_adaptive_delay(false, Some(&e.to_string().to_lowercase()));
                 Err(SodsVerifierError::RpcError(e.to_string()))
             }
        }
    }

    pub async fn fetch_logs_for_block(&self, block_number: u64) -> Result<Vec<Log>> {
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(logs) = cache.get(&block_number) {
                return Ok(logs.clone());
            }
        }

        let logs = self.fetch_with_backoff(block_number).await?;

        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(block_number, logs.clone());
        }

        Ok(logs)
    }

    pub async fn fetch_block_transactions(&self, block_number: u64) -> Result<Vec<ethers_core::types::Transaction>> {
        let block = self.provider
            .get_block_with_txs(block_number)
            .await
            .map_err(|e| SodsVerifierError::RpcError(e.to_string()))?
            .ok_or(SodsVerifierError::BlockOutOfRange(block_number))?;
            
        Ok(block.transactions)
    }

    async fn fetch_with_backoff(&self, block_number: u64) -> Result<Vec<Log>> {
        let filter = Filter::new()
            .from_block(BlockNumber::Number(block_number.into()))
            .to_block(BlockNumber::Number(block_number.into()));

        let mut last_error = None;
        let base_delays = [500, 1500, 4000]; // Fixed retries

        for (attempt, _fixed_delay) in base_delays.iter().enumerate() {
            // Apply Adaptive Delay BEFORE request
            let adaptive = self.adaptive_delay.load(std::sync::atomic::Ordering::Relaxed);
            if adaptive > MIN_ADAPTIVE_DELAY_MS {
                sleep(Duration::from_millis(adaptive)).await;
            }

            #[cfg(test)]
            {
                self.fetch_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }

            match self.provider.get_logs(&filter).await {
                Ok(logs) => {
                    self.update_adaptive_delay(true, None);
                    return Ok(logs);
                },
                Err(e) => {
                    let error = self.classify_error(e, block_number);
                    
                    // Update adaptive state ON FAILURE
                    // Extract inner message from classification
                    let msg = match &error {
                        SodsVerifierError::RpcError(m) => Some(m.as_str()),
                        _ => None,
                    };
                    self.update_adaptive_delay(false, msg);

                    if !Self::is_transient_error(&error) {
                        return Err(error);
                    }

                    last_error = Some(error);

                    if attempt < base_delays.len() - 1 {
                        let jitter = rand::random::<f64>() * JITTER_PERCENT * 2.0 - JITTER_PERCENT;
                        let delay = (base_delays[attempt] as f64 * (1.0 + jitter)) as u64;
                        sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(SodsVerifierError::RpcTimeout {
            attempts: base_delays.len() as u32,
        }))
    }

    /// Get the current cache size (for testing/monitoring).
    pub fn cache_len(&self) -> usize {
        self.cache.lock().unwrap().len()
    }

    /// Get the current adaptive delay in milliseconds.
    pub fn current_delay(&self) -> u64 {
        self.adaptive_delay.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Clear the cache.
    pub fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }

    /// Classify an ethers error into our error type.
    fn classify_error(
        &self,
        error: ethers_providers::ProviderError,
        block_number: u64,
    ) -> SodsVerifierError {
        let error_str = error.to_string().to_lowercase();

        // Common error patterns
        if error_str.contains("block not found")
            || error_str.contains("unknown block")
            || error_str.contains("header not found")
        {
            return SodsVerifierError::BlockOutOfRange(block_number);
        }

        if error_str.contains("rate limit") || error_str.contains("too many requests") {
            return SodsVerifierError::RpcError(
                "Rate limited by RPC provider. Try again later.".to_string(),
            );
        }

        SodsVerifierError::RpcError(error.to_string())
    }

    /// Check if an error is transient and should be retried.
    fn is_transient_error(error: &SodsVerifierError) -> bool {
        match error {
            SodsVerifierError::RpcError(msg) => {
                msg.contains("timeout")
                    || msg.contains("connection")
                    || msg.contains("rate limit")
                    || msg.contains("temporarily")
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_url() {
        let result = RpcClient::new("not-a-valid-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_url() {
        let result = RpcClient::new("https://sepolia.infura.io/v3/test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_size_default() {
        let client = RpcClient::new("https://example.com").unwrap();
        assert_eq!(client.cache_len(), 0);
    }

    #[tokio::test]
    async fn test_cache_hit() {
        // This test verifies cache behavior with a mock-like approach
        let client = RpcClient::new("https://rpc.sepolia.org").unwrap();

        // Manually insert into cache
        {
            let mut cache = client.cache.lock().unwrap();
            cache.put(12345, vec![]); // Empty logs for block 12345
        }

        // Fetch should return cached value without RPC call
        let result = client.fetch_logs_for_block(12345).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
