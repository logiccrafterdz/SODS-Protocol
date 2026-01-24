//! RPC client for fetching blockchain data.
//!
//! Wraps `ethers_providers::Provider` with LRU caching,
//! exponential backoff retry logic, and rate limit handling.

use ethers_core::types::{BlockNumber, Filter, Log, Address, H256, EIP1186ProofResponse};
use ethers_providers::{Http, Middleware, Provider};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;
use tokio::time::sleep;

const MIN_ADAPTIVE_DELAY_MS: u64 = 100;
const MAX_ADAPTIVE_DELAY_MS: u64 = 5000;
const JITTER_PERCENT: f64 = 0.1;

use crate::error::{Result, SodsVerifierError};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackoffProfile {
    Ethereum, // [500ms, 1500ms, 4000ms]
    L2,       // [1000ms, 3000ms, 8000ms]
}

impl BackoffProfile {
    pub fn delays(&self) -> &'static [u64] {
        match self {
            BackoffProfile::Ethereum => &[500, 1500, 4000],
            BackoffProfile::L2 => &[1000, 3000, 8000],
        }
    }
}

#[derive(Clone)]
pub struct RpcClient {
    providers: Vec<Arc<Provider<Http>>>,
    urls: Vec<String>,
    current_provider_index: Arc<std::sync::atomic::AtomicUsize>,
    cache: Arc<RwLock<HashMap<u64, Vec<Log>>>>,
    adaptive_delay: Arc<std::sync::atomic::AtomicU64>,
    backoff_profile: BackoffProfile,
    /// Total RPC fetch operations. Primarily for testing synchronization.
    pub fetch_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl RpcClient {
    pub fn new(rpc_urls: &[String]) -> Result<Self> {
        if rpc_urls.is_empty() {
            return Err(SodsVerifierError::RpcError("No RPC URLs provided".to_string()));
        }

        let mut providers = Vec::new();
        for url in rpc_urls {
            let provider = Provider::<Http>::try_from(url.as_str())
                .map_err(|e| SodsVerifierError::RpcError(format!("Invalid RPC URL {}: {}", url, e)))?;
            providers.push(Arc::new(provider));
        }

        Ok(Self {
            providers,
            urls: rpc_urls.to_vec(),
            current_provider_index: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            cache: Arc::new(RwLock::new(HashMap::new())),
            adaptive_delay: Arc::new(std::sync::atomic::AtomicU64::new(MIN_ADAPTIVE_DELAY_MS)),
            backoff_profile: BackoffProfile::Ethereum,
            fetch_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    /// Set the backoff profile (e.g., L2 for stricter rate limits).
    pub fn with_profile(mut self, profile: BackoffProfile) -> Self {
        self.backoff_profile = profile;
        self
    }

    fn current_provider(&self) -> Arc<Provider<Http>> {
        let idx = self.current_provider_index.load(std::sync::atomic::Ordering::Relaxed);
        self.providers[idx % self.providers.len()].clone()
    }


    fn switch_to_next_provider(&self) {
        let old_idx = self.current_provider_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let new_idx = (old_idx + 1) % self.providers.len();
        eprintln!("🔄 RPC Failover: Switching from {} to {}", self.urls[old_idx % self.urls.len()], self.urls[new_idx]);
    }

    /// Pre-flight health check.
    pub async fn health_check(&self) -> bool {
        self.current_provider().get_block_number().await.is_ok()
    }

    /// Update adaptive delay based on request outcome.
    fn update_adaptive_delay(&self, success: bool, error_kind: Option<&str>) {
        let current = self.adaptive_delay.load(std::sync::atomic::Ordering::Relaxed);
        
        let new_delay = if success {
            std::cmp::max(MIN_ADAPTIVE_DELAY_MS, (current as f64 * 0.9) as u64)
        } else if let Some(kind) = error_kind {
            if kind.contains("rate limit") || kind.contains("too many requests") || kind.contains("429") {
                std::cmp::min(MAX_ADAPTIVE_DELAY_MS, current * 2)
            } else if kind.contains("timeout") {
                std::cmp::min(MAX_ADAPTIVE_DELAY_MS, (current as f64 * 1.5) as u64)
            } else {
                current 
            }
        } else {
            current 
        };

        if new_delay != current {
            self.adaptive_delay.store(new_delay, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub async fn get_latest_block(&self) -> Result<u64> {
        let mut last_err = None;
        for _ in 0..self.providers.len() {
            match self.current_provider().get_block_number().await {
                Ok(n) => {
                    self.update_adaptive_delay(true, None);
                    return Ok(n.as_u64());
                },
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    self.update_adaptive_delay(false, Some(&err_str));
                    last_err = Some(SodsVerifierError::RpcError(e.to_string()));
                    self.switch_to_next_provider();
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub async fn get_proof(
        &self,
        address: Address,
        locations: Vec<H256>,
        block_number: Option<BlockNumber>,
    ) -> Result<EIP1186ProofResponse> {
        let mut last_err = None;
        for _ in 0..self.providers.len() {
            let block_id = block_number.map(ethers_core::types::BlockId::Number);
            match self.current_provider().get_proof(address, locations.clone(), block_id).await {
                Ok(p) => {
                    self.update_adaptive_delay(true, None);
                    return Ok(p);
                },
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    self.update_adaptive_delay(false, Some(&err_str));
                    last_err = Some(SodsVerifierError::RpcError(e.to_string()));
                    self.switch_to_next_provider();
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub async fn fetch_transaction_receipt(&self, tx_hash: H256) -> Result<ethers_core::types::TransactionReceipt> {
        let mut last_err = None;
        for _ in 0..self.providers.len() {
            match self.current_provider().get_transaction_receipt(tx_hash).await {
                Ok(Some(r)) => {
                    self.update_adaptive_delay(true, None);
                    return Ok(r);
                },
                Ok(None) => return Err(SodsVerifierError::RpcError("Receipt not found".into())),
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    self.update_adaptive_delay(false, Some(&err_str));
                    last_err = Some(SodsVerifierError::RpcError(e.to_string()));
                    self.switch_to_next_provider();
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub async fn fetch_logs_for_block(&self, block_number: u64) -> Result<Vec<Log>> {
        // 1. First check: Read lock (allows multiple concurrent readers)
        {
            let cache = self.cache.read().await;
            if let Some(logs) = cache.get(&block_number) {
                return Ok(logs.clone());
            }
        }

        // 2. Second check: Write lock with double-check (Stampede Prevention)
        let mut cache = self.cache.write().await;
        
        // Double-check: another request might have filled the cache while we were waiting for write lock
        if let Some(logs) = cache.get(&block_number) {
            return Ok(logs.clone());
        }

        // 3. Fetch from RPC (only one request per block reaches here)
        let logs = self.fetch_with_backoff(block_number, None).await?;

        // 4. Populate cache
        cache.insert(block_number, logs.clone());

        Ok(logs)
    }

    /// Fetch only logs matching specific topics for a block.
    pub async fn fetch_filtered_logs(&self, block_number: u64, topics: Vec<H256>) -> Result<Vec<Log>> {
        // Note: Filtered logs are NOT cached to avoid incomplete cache entries
        self.fetch_with_backoff(block_number, Some(topics)).await
    }

    pub async fn fetch_block_transactions(&self, block_number: u64) -> Result<Vec<ethers_core::types::Transaction>> {
        let mut last_err = None;
        for _ in 0..self.providers.len() {
            let provider = self.current_provider();
            match provider.get_block_with_txs(block_number).await {
                Ok(Some(b)) => {
                    self.update_adaptive_delay(true, None);
                    return Ok(b.transactions);
                }
                Ok(None) => return Err(SodsVerifierError::BlockOutOfRange(block_number)),
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    self.update_adaptive_delay(false, Some(&err_str));
                    last_err = Some(SodsVerifierError::RpcError(e.to_string()));
                    self.switch_to_next_provider();
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub async fn fetch_block_header(&self, block_number: u64) -> Result<crate::header_anchor::BlockHeader> {
        let mut last_err = None;
        for _ in 0..self.providers.len() {
            let provider = self.current_provider();
            match provider.get_block(block_number).await {
                Ok(Some(block)) => {
                    self.update_adaptive_delay(true, None);
                    return Ok(crate::header_anchor::BlockHeader {
                        number: block.number.map(|n| n.as_u64()).unwrap_or(block_number),
                        hash: block.hash.unwrap_or_default(),
                        receipts_root: block.receipts_root,
                        parent_beacon_block_root: block.parent_beacon_block_root,
                        timestamp: block.timestamp.as_u64(),
                        logs_bloom: block.logs_bloom.unwrap_or_default(),
                    });
                }
                Ok(None) => return Err(SodsVerifierError::HeaderFetchFailed(block_number)),
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    self.update_adaptive_delay(false, Some(&err_str));
                    last_err = Some(SodsVerifierError::RpcError(e.to_string()));
                    self.switch_to_next_provider();
                }
            }
        }
        Err(last_err.unwrap())
    }

    pub async fn fetch_contract_deployer(&self, contract_address: ethers_core::types::Address) -> Result<Option<ethers_core::types::Address>> {
        let mut last_err = None;
        let mut code_fetched = false;
        
        for _ in 0..self.providers.len() {
            let provider = self.current_provider();
            match provider.get_code(contract_address, None).await {
                Ok(code) => {
                    if code.is_empty() { return Ok(None); }
                    code_fetched = true;
                    break;
                }
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    self.update_adaptive_delay(false, Some(&err_str));
                    last_err = Some(SodsVerifierError::RpcError(e.to_string()));
                    self.switch_to_next_provider();
                }
            }
        }

        if !code_fetched { return Err(last_err.unwrap()); }

        // Search for deployment tx (expensive, limited to first 1000 blocks)
        for block_num in 0..1000u64 {
            let mut block = None;
            for _ in 0..self.providers.len() {
                let provider = self.current_provider();
                match provider.get_block_with_txs(block_num).await {
                    Ok(res) => { block = res; break; },
                    Err(_) => self.switch_to_next_provider(),
                }
            }
            
            let block = match block { Some(b) => b, None => continue };
            
            for (_i, tx) in block.transactions.iter().enumerate() {
                if tx.to.is_none() {
                    let mut receipt = None;
                    for _ in 0..self.providers.len() {
                        let provider = self.current_provider();
                        match provider.get_transaction_receipt(tx.hash).await {
                            Ok(res) => { receipt = res; break; },
                            Err(_) => self.switch_to_next_provider(),
                        }
                    }
                    
                    if let Some(r) = receipt {
                        if r.contract_address == Some(contract_address) {
                            return Ok(Some(tx.from));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }

    pub async fn fetch_block_receipts(&self, block_number: u64) -> Result<Vec<ethers_core::types::TransactionReceipt>> {
        let mut last_err = None;
        let mut block_transactions = Vec::new();

        for _ in 0..self.providers.len() {
            let provider = self.current_provider();
            match provider.get_block(block_number).await {
                Ok(Some(block)) => {
                    block_transactions = block.transactions;
                    break;
                }
                Ok(None) => return Err(SodsVerifierError::BlockOutOfRange(block_number)),
                Err(e) => {
                    let err_str = e.to_string().to_lowercase();
                    self.update_adaptive_delay(false, Some(&err_str));
                    last_err = Some(SodsVerifierError::RpcError(e.to_string()));
                    self.switch_to_next_provider();
                }
            }
        }

        if block_transactions.is_empty() && last_err.is_some() {
            return Err(last_err.unwrap());
        }

        if block_transactions.is_empty() {
            return Ok(Vec::new());
        }

        let mut receipts = Vec::with_capacity(block_transactions.len());
        for tx_hash in block_transactions {
            let mut matched_receipt = None;
            for _ in 0..self.providers.len() {
                let provider = self.current_provider();
                match provider.get_transaction_receipt(tx_hash).await {
                    Ok(Some(r)) => { matched_receipt = Some(r); break; },
                    Ok(None) => return Err(SodsVerifierError::ReceiptFetchFailed(block_number)),
                    Err(_) => self.switch_to_next_provider(),
                }
            }
            match matched_receipt {
                Some(r) => receipts.push(r),
                None => return Err(SodsVerifierError::ReceiptFetchFailed(block_number)),
            }
        }

        receipts.sort_by_key(|r| r.transaction_index);
        Ok(receipts)
    }

    async fn fetch_with_backoff(&self, block_number: u64, topics: Option<Vec<H256>>) -> Result<Vec<Log>> {
        let mut filter = Filter::new()
            .from_block(BlockNumber::Number(block_number.into()))
            .to_block(BlockNumber::Number(block_number.into()));

        if let Some(t) = topics {
            filter = filter.topic0(t); // Filter by topic0 (event signatures)
        }

        let mut last_error = None;
        let profile_delays = self.backoff_profile.delays();

        for _provider_attempt in 0..self.providers.len() {
            // Only retry 3 times if it's a rate limit. 
            // If it's a timeout or connection error, try only ONCE or TWICE then move to next provider immediately.
            let max_retries = profile_delays.len();
            
            for (attempt, _fixed_delay) in profile_delays.iter().enumerate() {
                let adaptive = self.adaptive_delay.load(std::sync::atomic::Ordering::Relaxed);
                if adaptive > MIN_ADAPTIVE_DELAY_MS {
                    sleep(Duration::from_millis(adaptive)).await;
                }

                self.fetch_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                match self.current_provider().get_logs(&filter).await {
                    Ok(logs) => {
                        self.update_adaptive_delay(true, None);
                        return Ok(logs);
                    },
                    Err(e) => {
                        let error = self.classify_error(e, block_number);
                        let msg = match &error {
                            SodsVerifierError::RpcError(m) => Some(m.as_str()),
                            _ => None,
                        };
                        self.update_adaptive_delay(false, msg);

                        if !Self::is_transient_error(&error) {
                            break; // Try next provider immediately
                        }
                        
                        // If it's a timeout or connection error (not a rate limit), 
                        // we switch provider earlier instead of exhausting all retries on a broken one.
                        let is_rate_limit = msg.map(|m| m.contains("rate")).unwrap_or(false);
                        if !is_rate_limit && attempt >= 1 {
                             break; // Failover sooner for connection issues
                        }

                        last_error = Some(error);

                        if attempt < max_retries - 1 {
                            let jitter = rand::random::<f64>() * JITTER_PERCENT * 2.0 - JITTER_PERCENT;
                            let delay = (profile_delays[attempt] as f64 * (1.0 + jitter)) as u64;
                            sleep(Duration::from_millis(delay)).await;
                        }
                    }
                }
            }
            // If all retries on this provider failed, switch to next
            self.switch_to_next_provider();
        }

        Err(last_error.unwrap_or(SodsVerifierError::RpcTimeout {
            attempts: (profile_delays.len() * self.providers.len()) as u32,
        }))
    }

    /// Get the current cache size (for testing/monitoring).
    pub async fn cache_len(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Get the current adaptive delay in milliseconds.
    pub fn current_delay(&self) -> u64 {
        self.adaptive_delay.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get the fetch count (total RPC calls). Primarily for internal verification.
    pub fn get_fetch_count(&self) -> usize {
        self.fetch_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Clear the cache.
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
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
        let urls = vec!["not-a-valid-url".to_string()];
        let result = RpcClient::new(&urls);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_url() {
        let urls = vec!["https://sepolia.infura.io/v3/test".to_string()];
        let result = RpcClient::new(&urls);
        assert!(result.is_ok());
    }

    #[test]
    fn test_backoff_profiles() {
        let eth = BackoffProfile::Ethereum;
        let l2 = BackoffProfile::L2;
        
        assert_eq!(eth.delays()[0], 500);
        assert_eq!(l2.delays()[0], 1000);
        assert!(l2.delays()[2] > eth.delays()[2]);
    }

    #[test]
    fn test_cache_size_default() {
        let urls = vec!["https://example.com".to_string()];
        let client = RpcClient::new(&urls).unwrap();
        assert_eq!(client.cache_len(), 0);
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let urls = vec!["https://rpc.sepolia.org".to_string()];
        let client = RpcClient::new(&urls).unwrap();

        {
            let mut cache = client.cache.write().await;
            cache.insert(12345, vec![]); 
        }

        let result = client.fetch_logs_for_block(12345).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }
}
