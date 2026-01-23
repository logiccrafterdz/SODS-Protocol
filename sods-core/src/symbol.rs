//! Behavioral symbol representation.
//!
//! This module defines the `BehavioralSymbol` struct which represents
//! a parsed behavioral event extracted from EVM logs.

use ethers_core::types::{Address, U256, H256};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A behavioral symbol extracted from an EVM log event.
///
/// Represents an atomic blockchain operation identified by its symbol code
/// (e.g., "Tf" for Transfer, "Sw" for Swap) along with its position in the
/// block and rich contextual metadata for threat detection.
///
/// # Ordering
///
/// Symbols are ordered canonically per RFC §4.4.2:
/// 1. Primary: `log_index` (ascending)
/// 2. Secondary: `symbol` (lexicographic, as tie-breaker)
///
/// This ensures deterministic BMT construction across all implementations.
///
/// # Example
///
/// ```rust
/// use sods_core::BehavioralSymbol;
/// use ethers_core::types::{Address, U256};
///
/// let symbol = BehavioralSymbol::new("Tf", 42);
/// assert_eq!(symbol.symbol(), "Tf");
/// assert_eq!(symbol.log_index(), 42);
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BehavioralSymbol {
    /// The symbolic code (e.g., "Tf", "Sw", "Dep", "Wdw", "MintNFT")
    pub symbol: String,

    /// Position of this event in the block (0-indexed)
    pub log_index: u32,

    /// Sender address (context)
    pub from: Address,

    /// Receiver address (context)
    pub to: Address,

    /// Value or Amount involved (context)
    pub value: U256,

    /// Token ID (for NFT events)
    pub token_id: Option<U256>,

    /// Heuristic flag: true if `from` matches contract deployer (Rug Pull risk)
    pub is_from_deployer: bool,

    /// Transaction Hash (Causality)
    pub tx_hash: H256,
    
    /// Nonce of the sender (Causality - EOA)
    pub nonce: u64,

    /// Call sequence / Trace index (Causality - Contract)
    pub call_sequence: u32,

    /// Optional raw contextual data (legacy/compat)
    pub metadata: Vec<u8>,

    /// Account Abstraction UserOperation Hash (ERC-4337)
    pub user_op_hash: Option<H256>,

    /// Permit2 expiration/deadline (timestamp)
    pub permit_deadline: Option<u64>,

    /// CoW Swap solver address
    pub solver: Option<Address>,
}

impl BehavioralSymbol {
    /// Create a new behavioral symbol with default context.
    pub fn new(symbol: impl Into<String>, log_index: u32) -> Self {
        Self {
            symbol: symbol.into(),
            log_index,
            from: Address::zero(),
            to: Address::zero(),
            value: U256::zero(),
            token_id: None,
            is_from_deployer: false,
            tx_hash: H256::zero(),
            nonce: 0,
            call_sequence: 0,
            metadata: vec![],
            user_op_hash: None,
            permit_deadline: None,
            solver: None,
        }
    }

    /// Set contextual metadata (Builder pattern).
    pub fn with_context(
        mut self, 
        from: Address, 
        to: Address, 
        value: U256, 
        token_id: Option<U256>
    ) -> Self {
        self.from = from;
        self.to = to;
        self.value = value;
        self.token_id = token_id;
        self
    }

    /// Set causality metadata (Builder pattern).
    pub fn with_causality(mut self, tx_hash: H256, nonce: u64, call_sequence: u32) -> Self {
        self.tx_hash = tx_hash;
        self.nonce = nonce;
        self.call_sequence = call_sequence;
        self
    }

    /// Set Account Abstraction context (Builder pattern).
    pub fn with_aa_context(mut self, user_op_hash: H256) -> Self {
        self.user_op_hash = Some(user_op_hash);
        self
    }

    /// Set Permit2 context (Builder pattern).
    pub fn with_permit2_context(mut self, deadline: u64) -> Self {
        self.permit_deadline = Some(deadline);
        self
    }

    /// Set CoW Swap context (Builder pattern).
    pub fn with_cow_context(mut self, solver: Address) -> Self {
        self.solver = Some(solver);
        self
    }

    /// Returns the symbolic code.
    #[inline]
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Returns the log index (position in block).
    #[inline]
    pub fn log_index(&self) -> u32 {
        self.log_index
    }

    /// Returns the metadata bytes.
    #[inline]
    pub fn metadata(&self) -> &[u8] {
        &self.metadata
    }

    /// Compute the leaf hash for this symbol.
    ///
    /// In minimal mode: `SHA256(symbol_bytes)`
    /// In full mode: `SHA256(symbol_bytes || metadata || causality)`
    pub fn leaf_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(self.symbol.as_bytes());

        // Include metadata if present (full mode)
        if !self.metadata.is_empty() {
            hasher.update(&self.metadata);
        }

        // Include causality fields if non-zero (to affect hash)
        if self.tx_hash != H256::zero() {
             hasher.update(self.tx_hash.as_bytes());
             hasher.update(&self.nonce.to_be_bytes());
             hasher.update(&self.call_sequence.to_be_bytes());
        }

        hasher.finalize().into()
    }

    /// Compute the leaf hash for this symbol using Keccak256 (EVM compatible).
    ///
    /// Formula: `keccak256(abi.encodePacked(symbol, log_index))`
    pub fn leaf_hash_keccak(&self) -> [u8; 32] {
        use tiny_keccak::{Hasher, Keccak};

        let mut hasher = Keccak::v256();
        hasher.update(self.symbol.as_bytes());
        hasher.update(&self.log_index.to_be_bytes());
        
        let mut output = [0u8; 32];
        hasher.finalize(&mut output);
        output
    }
}

impl Ord for BehavioralSymbol {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary sort: log_index (ascending)
        // Secondary sort: symbol (lexicographic, as tie-breaker)
        match self.log_index.cmp(&other.log_index) {
            Ordering::Equal => self.symbol.cmp(&other.symbol),
            ord => ord,
        }
    }
}

impl PartialOrd for BehavioralSymbol {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_creation() {
        let sym = BehavioralSymbol::new("Tf", 42)
            .with_context(Address::zero(), Address::zero(), U256::from(100), None);
            
        assert_eq!(sym.symbol(), "Tf");
        assert_eq!(sym.log_index(), 42);
        assert_eq!(sym.value, U256::from(100));
    }

    #[test]
    fn test_symbol_ordering_by_log_index() {
        let sym1 = BehavioralSymbol::new("Tf", 1);
        let sym2 = BehavioralSymbol::new("Tf", 2);
        assert!(sym1 < sym2);
    }

    #[test]
    fn test_symbol_ordering_tiebreaker() {
        let sym_a = BehavioralSymbol::new("Dep", 5);
        let sym_b = BehavioralSymbol::new("Tf", 5);
        // "Dep" < "Tf" lexicographically
        assert!(sym_a < sym_b);
    }

    #[test]
    fn test_leaf_hash_minimal() {
        let sym = BehavioralSymbol::new("Tf", 0);
        let hash = sym.leaf_hash();
        assert_eq!(hash.len(), 32);
        // Verify it's consistent
        assert_eq!(sym.leaf_hash(), hash);
    }

    #[test]
    fn test_sorting_mixed_symbols() {
        let mut symbols = vec![
            BehavioralSymbol::new("Wdw", 10),
            BehavioralSymbol::new("Tf", 5),
            BehavioralSymbol::new("Dep", 5),
            BehavioralSymbol::new("Tf", 1),
        ];
        symbols.sort();

        assert_eq!(symbols[0].log_index(), 1);
        assert_eq!(symbols[0].symbol(), "Tf");

        assert_eq!(symbols[1].log_index(), 5);
        assert_eq!(symbols[1].symbol(), "Dep"); // "Dep" < "Tf"

        assert_eq!(symbols[2].log_index(), 5);
        assert_eq!(symbols[2].symbol(), "Tf");

        assert_eq!(symbols[3].log_index(), 10);
        assert_eq!(symbols[3].symbol(), "Wdw");
    }
}
