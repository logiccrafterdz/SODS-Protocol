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

/// Exponential backoff delays in milliseconds.
const BACKOFF_DELAYS_MS: [u64; 3] = [500, 1500, 4000];

/// Jitter percentage for backoff (±10%).
const JITTER_PERCENT: f64 = 0.1;

/// RPC client for fetching blockchain data.
///
/// Features:
/// - LRU cache for fetched logs (configurable via `SODS_RPC_CACHE_SIZE` env var)
/// - Exponential backoff with jitter for retries
/// - Smart error classification
///
/// # Example
///
/// ```rust,no_run
/// use sods_verifier::RpcClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = RpcClient::new("https://sepolia.infura.io/v3/YOUR_KEY")?;
///     
///     // First call fetches from RPC
///     let logs1 = client.fetch_logs_for_block(10002322).await?;
///     
///     // Second call returns cached data
///     let logs2 = client.fetch_logs_for_block(10002322).await?;
///     
///     println!("Found {} logs", logs1.len());
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct RpcClient {
    provider: Arc<Provider<Http>>,
    cache: Arc<Mutex<LruCache<u64, Vec<Log>>>>,
    #[cfg(test)]
    pub(crate) fetch_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl RpcClient {
    /// Create a new RPC client from a URL.
    ///
    /// Cache size can be configured via the `SODS_RPC_CACHE_SIZE` environment variable.
    /// Default is 100 blocks.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The HTTP RPC endpoint URL
    ///
    /// # Errors
    ///
    /// Returns `RpcError` if the URL is invalid.
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
            #[cfg(test)]
            fetch_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    /// Fetch the latest block number.
    pub async fn get_latest_block(&self) -> Result<u64> {
        self.provider
            .get_block_number()
            .await
            .map(|n| n.as_u64())
            .map_err(|e| SodsVerifierError::RpcError(e.to_string()))
    }

    /// Fetch all logs for a specific block.
    ///
    /// Returns cached data if available, otherwise fetches from RPC
    /// with exponential backoff retry logic.
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number to fetch logs for
    ///
    /// # Returns
    ///
    /// A vector of logs from the block.
    ///
    /// # Errors
    ///
    /// - `RpcError` on network failure
    /// - `RpcTimeout` after max retry attempts
    /// - `BlockOutOfRange` if block doesn't exist
    pub async fn fetch_logs_for_block(&self, block_number: u64) -> Result<Vec<Log>> {
        // Check cache first
        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(logs) = cache.get(&block_number) {
                return Ok(logs.clone());
            }
        }

        // Fetch from RPC with exponential backoff
        let logs = self.fetch_with_backoff(block_number).await?;

        // Cache the result
        {
            let mut cache = self.cache.lock().unwrap();
            cache.put(block_number, logs.clone());
        }

        Ok(logs)
    }

    /// Fetch logs with exponential backoff retry.
    async fn fetch_with_backoff(&self, block_number: u64) -> Result<Vec<Log>> {
        let filter = Filter::new()
            .from_block(BlockNumber::Number(block_number.into()))
            .to_block(BlockNumber::Number(block_number.into()));

        let mut last_error = None;

        for (attempt, base_delay) in BACKOFF_DELAYS_MS.iter().enumerate() {
            #[cfg(test)]
            {
                self.fetch_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }

            match self.provider.get_logs(&filter).await {
                Ok(logs) => return Ok(logs),
                Err(e) => {
                    let error = self.classify_error(e, block_number);

                    // Don't retry on non-transient errors
                    if !Self::is_transient_error(&error) {
                        return Err(error);
                    }

                    last_error = Some(error);

                    // Apply jitter and sleep (except on last attempt)
                    if attempt < BACKOFF_DELAYS_MS.len() - 1 {
                        let jitter = rand::random::<f64>() * JITTER_PERCENT * 2.0 - JITTER_PERCENT;
                        let actual_delay = (*base_delay as f64 * (1.0 + jitter)) as u64;
                        sleep(Duration::from_millis(actual_delay)).await;
                    }
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or(SodsVerifierError::RpcTimeout {
            attempts: BACKOFF_DELAYS_MS.len() as u32,
        }))
    }

    /// Get the current cache size (for testing/monitoring).
    pub fn cache_len(&self) -> usize {
        self.cache.lock().unwrap().len()
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
