//! Block verifier — the main public API.
//!
//! Provides a simple interface for verifying behavioral symbols
//! in on-chain blocks using the SODS protocol.

use std::time::Instant;

use sods_core::{BehavioralMerkleTree, BehavioralSymbol, SymbolDictionary};

use crate::error::{Result, SodsVerifierError};
use crate::query::QueryParser;
use crate::result::VerificationResult;
use crate::rpc::RpcClient;

/// Block verifier for SODS behavioral symbol verification.
///
/// The main entry point for verifying symbols in on-chain blocks.
/// Uses public RPC endpoints to fetch block data and `sods-core`
/// to build Behavioral Merkle Trees and generate proofs.
///
/// # Example
///
/// ```rust,no_run
/// use sods_verifier::BlockVerifier;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let verifier = BlockVerifier::new("https://sepolia.infura.io/v3/YOUR_KEY")?;
///     
///     let result = verifier
///         .verify_symbol_in_block("Dep", 10002322)
///         .await?;
///
///     if result.is_verified {
///         println!("Symbol verified with {} byte proof", result.proof_size_bytes);
///     }
///
///     Ok(())
/// }
/// ```
pub struct BlockVerifier {
    rpc_client: RpcClient,
    query_parser: QueryParser,
    dictionary: SymbolDictionary,
}

impl BlockVerifier {
    /// Create a new block verifier.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - The HTTP RPC endpoint URL (e.g., Infura, Alchemy)
    ///
    /// # Errors
    ///
    /// Returns `RpcError` if the URL is invalid.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sods_verifier::BlockVerifier;
    ///
    /// let verifier = BlockVerifier::new("https://sepolia.infura.io/v3/YOUR_KEY")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(rpc_url: &str) -> Result<Self> {
        let rpc_client = RpcClient::new(rpc_url)?;

        Ok(Self {
            rpc_client,
            query_parser: QueryParser::new(),
            dictionary: SymbolDictionary::default(),
        })
    }

    /// Verify if a behavioral symbol exists in a block.
    ///
    /// This method:
    /// 1. Validates the symbol query
    /// 2. Fetches all logs for the block via RPC
    /// 3. Parses logs into behavioral symbols
    /// 4. Builds a Behavioral Merkle Tree
    /// 5. Generates a proof if the symbol is found
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbol to verify (e.g., "Tf", "Dep", "Wdw")
    /// * `block_number` - The block number to search
    ///
    /// # Returns
    ///
    /// A `VerificationResult` containing:
    /// - `is_verified`: true if symbol was found
    /// - `proof_size_bytes`: size of the Merkle proof
    /// - `occurrences`: number of times the symbol appears
    /// - Timing metrics for performance analysis
    ///
    /// # Errors
    ///
    /// - `UnsupportedSymbol` if the symbol is not in the registry
    /// - `RpcError` on network failure
    /// - `BlockOutOfRange` if the block doesn't exist
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use sods_verifier::BlockVerifier;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let verifier = BlockVerifier::new("https://sepolia.infura.io/v3/YOUR_KEY")?;
    ///     
    ///     let result = verifier
    ///         .verify_symbol_in_block("Dep", 10002322)
    ///         .await?;
    ///
    ///     println!("{}", result);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn verify_symbol_in_block(
        &self,
        symbol: &str,
        block_number: u64,
    ) -> Result<VerificationResult> {
        let total_start = Instant::now();

        // Step 1: Validate symbol
        self.query_parser.validate_symbol(symbol)?;

        // Step 2: Fetch logs AND transactions (for causality)
        let rpc_start = Instant::now();
        // Parallel fetch could be better but keeping simple for now
        let logs_fut = self.rpc_client.fetch_logs_for_block(block_number);
        let txs_fut = self.rpc_client.fetch_block_transactions(block_number);
        
        let (logs, txs) = tokio::try_join!(logs_fut, txs_fut)?;
        let rpc_fetch_time = rpc_start.elapsed();

        // Build Tx Lookup Map: TxHash -> (Nonce, From)
        use std::collections::HashMap;
        let tx_map: HashMap<_, _> = txs.iter()
            .map(|tx| (tx.hash, (tx.nonce, tx.from)))
            .collect();

        // Step 3: Parse logs to symbols
        let verify_start = Instant::now();
        let symbols = self.parse_logs_to_symbols(&logs, &tx_map);

        // Handle empty block
        if symbols.is_empty() {
            return Ok(VerificationResult::not_found(
                symbol.to_string(),
                block_number,
                None,
                rpc_fetch_time,
                total_start.elapsed(),
            ));
        }

        // Step 4: Build BMT (standard) - Causal checking happens in pattern verification
        // For single symbol verification, standard BMT is fine.
        let bmt = BehavioralMerkleTree::new(symbols.clone());
        let root = bmt.root();

        // Step 5: Count occurrences and find first match
        let occurrences = symbols
            .iter()
            .filter(|s| s.symbol() == symbol)
            .count();

        if occurrences == 0 {
            return Ok(VerificationResult::not_found(
                symbol.to_string(),
                block_number,
                Some(root),
                rpc_fetch_time,
                total_start.elapsed(),
            ));
        }

        // Find first occurrence and generate proof
        let first_match = symbols
            .iter()
            .find(|s| s.symbol() == symbol)
            .expect("occurrences > 0");

        let proof = bmt
            .generate_proof(symbol, first_match.log_index())
            .ok_or_else(|| SodsVerifierError::SymbolNotFound {
                symbol: symbol.to_string(),
                block_number,
            })?;

        // Calculate Confidence Score
        let mut score = 0.5;
        if first_match.from != ethers_core::types::Address::zero() { score += 0.2; }
        if first_match.is_from_deployer { score += 0.3; }
        if !first_match.value.is_zero() { score += 0.1; }
        if score > 1.0 { score = 1.0; }

        let verification_time = verify_start.elapsed();
        let total_time = total_start.elapsed();

        Ok(VerificationResult::success(
            symbol.to_string(),
            block_number,
            proof.size(),
            root,
            occurrences,
            score,
            verification_time,
            rpc_fetch_time,
            total_time,
        ))
    }

    /// Parse RPC logs into behavioral symbols.
    fn parse_logs_to_symbols(
        &self, 
        logs: &[ethers_core::types::Log],
        tx_map: &std::collections::HashMap<ethers_core::types::H256, (ethers_core::types::U256, ethers_core::types::Address)>
    ) -> Vec<BehavioralSymbol> {
        logs.iter()
            .filter_map(|log| {
                let mut sym = self.dictionary.parse_log(log)?;
                
                // Enrich with causal data if tx exists
                if let Some(tx_hash) = log.transaction_hash {
                    if let Some((nonce, from)) = tx_map.get(&tx_hash) {
                         // Use log_index as call_sequence for intra-tx ordering
                         sym = sym.with_causality(
                             tx_hash, 
                             nonce.as_u64(), 
                             log.log_index.map(|i| i.as_u32()).unwrap_or(0)
                         );
                         // If the symbol context 'from' is 0x0 (not extracted from log topics),
                         // we can fallback to tx.origin (though semantically different, helpful for causality grouping)
                         // But for now, let's keep 'from' as event-specific.
                         // Actually, Causal Tree sorts by sym.from. If sym.from is 0x0, it breaks grouping.
                         // So we should probably set sym.from to tx.from if it's empty?
                         // The user said: "Group symbols by transaction origin (from address)"
                         // If the event doesn't explicitly have a 'from' (like Swap), we should attr it to the tx sender.
                         if sym.from == ethers_core::types::Address::zero() {
                             sym.from = *from;
                         }
                    }
                }
                Some(sym)
            })
            .collect()
    }

    /// Get the symbol dictionary used for parsing.
    pub fn dictionary(&self) -> &SymbolDictionary {
        &self.dictionary
    }
    /// Get the latest verified block number from the chain.
    pub async fn get_latest_block(&self) -> Result<u64> {
        self.rpc_client.get_latest_block().await
    }

    /// Fetch all behavioral symbols for a block.
    /// 
    /// Useful for pattern matching or manual inspection.
    pub async fn fetch_block_symbols(&self, block_number: u64) -> Result<Vec<BehavioralSymbol>> {
        let logs_fut = self.rpc_client.fetch_logs_for_block(block_number);
        let txs_fut = self.rpc_client.fetch_block_transactions(block_number);
        
        let (logs, txs) = tokio::try_join!(logs_fut, txs_fut)?;
        
        let tx_map: std::collections::HashMap<_, _> = txs.iter()
            .map(|tx| (tx.hash, (tx.nonce, tx.from)))
            .collect();

        Ok(self.parse_logs_to_symbols(&logs, &tx_map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        let result = BlockVerifier::new("https://sepolia.infura.io/v3/test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_verifier_url() {
        let result = BlockVerifier::new("not-a-url");
        assert!(result.is_err());
    }
}
