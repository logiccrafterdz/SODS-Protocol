//! Behavioral symbol representation.
//!
//! This module defines the `BehavioralSymbol` struct which represents
//! a parsed behavioral event extracted from EVM logs.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A behavioral symbol extracted from an EVM log event.
///
/// Represents an atomic blockchain operation identified by its symbol code
/// (e.g., "Tf" for Transfer, "Sw" for Swap) along with its position in the
/// block and optional metadata.
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
///
/// let symbol = BehavioralSymbol::new("Tf", 42, vec![]);
/// assert_eq!(symbol.symbol(), "Tf");
/// assert_eq!(symbol.log_index(), 42);
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BehavioralSymbol {
    /// The symbolic code (e.g., "Tf", "Sw", "Dep", "Wdw")
    symbol: String,

    /// Position of this event in the block (0-indexed)
    log_index: u32,

    /// Optional contextual data (addresses, amounts, etc.)
    /// In minimal mode, this is empty. In full mode, contains CBOR-encoded metadata.
    metadata: Vec<u8>,
}

impl BehavioralSymbol {
    /// Create a new behavioral symbol.
    ///
    /// # Arguments
    ///
    /// * `symbol` - The symbolic code (e.g., "Tf", "Dep")
    /// * `log_index` - Position in the block
    /// * `metadata` - Optional contextual data
    ///
    /// # Example
    ///
    /// ```rust
    /// use sods_core::BehavioralSymbol;
    ///
    /// let sym = BehavioralSymbol::new("Tf", 0, vec![]);
    /// ```
    pub fn new(symbol: impl Into<String>, log_index: u32, metadata: Vec<u8>) -> Self {
        Self {
            symbol: symbol.into(),
            log_index,
            metadata,
        }
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
    /// In full mode: `SHA256(symbol_bytes || metadata)`
    pub fn leaf_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(self.symbol.as_bytes());

        // Include metadata if present (full mode)
        if !self.metadata.is_empty() {
            hasher.update(&self.metadata);
        }

        hasher.finalize().into()
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
        let sym = BehavioralSymbol::new("Tf", 42, vec![1, 2, 3]);
        assert_eq!(sym.symbol(), "Tf");
        assert_eq!(sym.log_index(), 42);
        assert_eq!(sym.metadata(), &[1, 2, 3]);
    }

    #[test]
    fn test_symbol_ordering_by_log_index() {
        let sym1 = BehavioralSymbol::new("Tf", 1, vec![]);
        let sym2 = BehavioralSymbol::new("Tf", 2, vec![]);
        assert!(sym1 < sym2);
    }

    #[test]
    fn test_symbol_ordering_tiebreaker() {
        let sym_a = BehavioralSymbol::new("Dep", 5, vec![]);
        let sym_b = BehavioralSymbol::new("Tf", 5, vec![]);
        // "Dep" < "Tf" lexicographically
        assert!(sym_a < sym_b);
    }

    #[test]
    fn test_leaf_hash_minimal() {
        let sym = BehavioralSymbol::new("Tf", 0, vec![]);
        let hash = sym.leaf_hash();
        assert_eq!(hash.len(), 32);
        // Verify it's consistent
        assert_eq!(sym.leaf_hash(), hash);
    }

    #[test]
    fn test_sorting_mixed_symbols() {
        let mut symbols = vec![
            BehavioralSymbol::new("Wdw", 10, vec![]),
            BehavioralSymbol::new("Tf", 5, vec![]),
            BehavioralSymbol::new("Dep", 5, vec![]),
            BehavioralSymbol::new("Tf", 1, vec![]),
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
