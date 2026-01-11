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

        // Step 2: Fetch logs
        let rpc_start = Instant::now();
        let logs = self.rpc_client.fetch_logs_for_block(block_number).await?;
        let rpc_fetch_time = rpc_start.elapsed();

        // Step 3: Parse logs to symbols
        let verify_start = Instant::now();
        let symbols = self.parse_logs_to_symbols(&logs);

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

        // Step 4: Build BMT
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

        let verification_time = verify_start.elapsed();
        let total_time = total_start.elapsed();

        // Step 6: Return result
        Ok(VerificationResult::success(
            symbol.to_string(),
            block_number,
            proof.size(),
            root,
            occurrences,
            verification_time,
            rpc_fetch_time,
            total_time,
        ))
    }

    /// Parse RPC logs into behavioral symbols.
    fn parse_logs_to_symbols(&self, logs: &[ethers_core::types::Log]) -> Vec<BehavioralSymbol> {
        logs.iter()
            .filter_map(|log| self.dictionary.parse_log(log))
            .collect()
    }

    /// Get the symbol dictionary used for parsing.
    pub fn dictionary(&self) -> &SymbolDictionary {
        &self.dictionary
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
