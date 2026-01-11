//! RPC client for fetching blockchain data.
//!
//! Wraps `ethers_providers::Provider` with retry logic and
//! rate limit handling.

use ethers_providers::{Http, Middleware, Provider};
use ethers_core::types::{BlockNumber, Filter, Log};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use crate::error::{Result, SodsVerifierError};

/// Maximum number of retry attempts for RPC calls.
const MAX_RETRIES: u32 = 2;

/// Backoff duration between retries.
const RETRY_BACKOFF: Duration = Duration::from_millis(500);

/// RPC client for fetching blockchain data.
///
/// Wraps an Ethereum JSON-RPC provider with automatic retry
/// logic for transient errors.
///
/// # Example
///
/// ```rust,no_run
/// use sods_verifier::RpcClient;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = RpcClient::new("https://sepolia.infura.io/v3/YOUR_KEY")?;
///     let logs = client.fetch_logs_for_block(10002322).await?;
///     println!("Found {} logs", logs.len());
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct RpcClient {
    provider: Arc<Provider<Http>>,
}

impl RpcClient {
    /// Create a new RPC client from a URL.
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

        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    /// Fetch all logs for a specific block.
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
        let filter = Filter::new()
            .from_block(BlockNumber::Number(block_number.into()))
            .to_block(BlockNumber::Number(block_number.into()));

        self.execute_with_retry(|| async {
            self.provider
                .get_logs(&filter)
                .await
                .map_err(|e| self.classify_error(e, block_number))
        })
        .await
    }

    /// Execute an async operation with retry logic.
    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=MAX_RETRIES {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // Don't retry on non-transient errors
                    if !Self::is_transient_error(&e) {
                        return Err(e);
                    }

                    last_error = Some(e);

                    // Backoff before retry (except on last attempt)
                    if attempt < MAX_RETRIES {
                        sleep(RETRY_BACKOFF * (attempt + 1)).await;
                    }
                }
            }
        }

        // All retries exhausted
        Err(last_error.unwrap_or(SodsVerifierError::RpcTimeout {
            attempts: MAX_RETRIES + 1,
        }))
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
            return SodsVerifierError::RpcError(format!(
                "Rate limited by RPC provider. Try again later."
            ));
        }

        SodsVerifierError::RpcError(error.to_string())
    }

    /// Check if an error is transient and should be retried.
    fn is_transient_error(error: &SodsVerifierError) -> bool {
        matches!(
            error,
            SodsVerifierError::RpcError(msg)
                if msg.contains("timeout")
                    || msg.contains("connection")
                    || msg.contains("rate limit")
                    || msg.contains("temporarily")
        )
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
}
