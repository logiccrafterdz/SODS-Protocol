//! Symbol dictionary for mapping EVM event topics to behavioral symbols.
//!
//! This module provides the `SymbolDictionary` which maps known event topic
//! hashes (keccak256 of event signatures) to their corresponding behavioral
//! symbol codes.

use ethers_core::types::{Log, H256};
use std::collections::HashMap;

use crate::symbol::BehavioralSymbol;

// ============================================================================
// Core Event Topic Hashes (keccak256 of event signatures)
// ============================================================================

/// ERC20 Transfer(address indexed from, address indexed to, uint256 value)
const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

/// WETH Deposit(address indexed dst, uint256 wad)
const DEPOSIT_TOPIC: &str = "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c";

/// WETH Withdrawal(address indexed src, uint256 wad)
const WITHDRAWAL_TOPIC: &str = "0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65";

/// Uniswap V2 Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)
const SWAP_V2_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";

/// Uniswap V2 Mint(address indexed sender, uint256 amount0, uint256 amount1) - Add Liquidity
const MINT_V2_TOPIC: &str = "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26391d14d59cf6ad";

/// Uniswap V2 Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to) - Remove Liquidity
const BURN_V2_TOPIC: &str = "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496";

// ============================================================================
// Symbol Dictionary
// ============================================================================

/// Dictionary mapping EVM event topics to behavioral symbol codes.
///
/// The dictionary comes pre-loaded with core immutable symbols as defined
/// in RFC §3.3. Custom symbols can be added for extensibility.
///
/// # Core Symbols
///
/// | Symbol | Event | Description |
/// |--------|-------|-------------|
/// | `Tf` | Transfer | ERC20 token transfer |
/// | `Dep` | Deposit | WETH deposit (wrap ETH) |
/// | `Wdw` | Withdrawal | WETH withdrawal (unwrap ETH) |
/// | `Sw` | Swap | Uniswap V2/V3 swap |
/// | `LP+` | Mint | Add liquidity |
/// | `LP-` | Burn | Remove liquidity |
///
/// # Example
///
/// ```rust
/// use sods_core::SymbolDictionary;
/// use ethers_core::types::H256;
///
/// let dict = SymbolDictionary::default();
///
/// // Look up Transfer topic
/// let topic = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
///     .parse::<H256>()
///     .unwrap();
/// assert_eq!(dict.symbol_for_topic(topic), Some("Tf"));
/// ```
#[derive(Debug, Clone)]
pub struct SymbolDictionary {
    /// Mapping from topic hash to symbol code
    registry: HashMap<H256, &'static str>,
}

impl Default for SymbolDictionary {
    /// Create a dictionary with all core immutable symbols.
    fn default() -> Self {
        let mut registry = HashMap::new();

        // Parse and insert core symbols
        let core_symbols: &[(&str, &'static str)] = &[
            (TRANSFER_TOPIC, "Tf"),
            (DEPOSIT_TOPIC, "Dep"),
            (WITHDRAWAL_TOPIC, "Wdw"),
            (SWAP_V2_TOPIC, "Sw"),
            (MINT_V2_TOPIC, "LP+"),
            (BURN_V2_TOPIC, "LP-"),
        ];

        for (topic_hex, symbol) in core_symbols {
            if let Ok(topic) = topic_hex.parse::<H256>() {
                registry.insert(topic, *symbol);
            }
        }

        Self { registry }
    }
}

impl SymbolDictionary {
    /// Create an empty dictionary (no pre-loaded symbols).
    pub fn empty() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    /// Look up the symbol for a given event topic.
    ///
    /// Returns `None` if the topic is not in the registry.
    ///
    /// # Arguments
    ///
    /// * `topic` - The event topic hash (keccak256 of event signature)
    #[inline]
    pub fn symbol_for_topic(&self, topic: H256) -> Option<&'static str> {
        self.registry.get(&topic).copied()
    }

    /// Register a custom symbol for a topic.
    ///
    /// This allows extending the dictionary with protocol-specific events.
    /// Note: This will overwrite any existing mapping for the topic.
    ///
    /// # Arguments
    ///
    /// * `topic` - The event topic hash
    /// * `symbol` - The symbol code to assign
    pub fn register_custom(&mut self, topic: H256, symbol: &'static str) {
        self.registry.insert(topic, symbol);
    }

    /// Parse an EVM log into a behavioral symbol.
    ///
    /// Returns `None` if the log's topic is not in the registry.
    ///
    /// # Arguments
    ///
    /// * `log` - The EVM log to parse
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let dict = SymbolDictionary::default();
    /// if let Some(symbol) = dict.parse_log(&log) {
    ///     println!("Found symbol: {}", symbol.symbol());
    /// }
    /// ```
    pub fn parse_log(&self, log: &Log) -> Option<BehavioralSymbol> {
        // Get topic[0] (event signature hash)
        let topic = log.topics.first()?;

        // Look up symbol
        let symbol_code = self.symbol_for_topic(*topic)?;

        // Extract log index
        let log_index = log.log_index.map(|i| i.as_u32()).unwrap_or(0);

        // In minimal mode, we don't include metadata in the hash
        // Metadata could be added for full mode (address, topics, data)
        Some(BehavioralSymbol::new(symbol_code, log_index, vec![]))
    }

    /// Returns the number of registered symbols.
    #[inline]
    pub fn len(&self) -> usize {
        self.registry.len()
    }

    /// Returns true if the dictionary has no registered symbols.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_has_core_symbols() {
        let dict = SymbolDictionary::default();
        assert_eq!(dict.len(), 6);
    }

    #[test]
    fn test_transfer_lookup() {
        let dict = SymbolDictionary::default();
        let topic = TRANSFER_TOPIC.parse::<H256>().unwrap();
        assert_eq!(dict.symbol_for_topic(topic), Some("Tf"));
    }

    #[test]
    fn test_deposit_lookup() {
        let dict = SymbolDictionary::default();
        let topic = DEPOSIT_TOPIC.parse::<H256>().unwrap();
        assert_eq!(dict.symbol_for_topic(topic), Some("Dep"));
    }

    #[test]
    fn test_withdrawal_lookup() {
        let dict = SymbolDictionary::default();
        let topic = WITHDRAWAL_TOPIC.parse::<H256>().unwrap();
        assert_eq!(dict.symbol_for_topic(topic), Some("Wdw"));
    }

    #[test]
    fn test_swap_lookup() {
        let dict = SymbolDictionary::default();
        let topic = SWAP_V2_TOPIC.parse::<H256>().unwrap();
        assert_eq!(dict.symbol_for_topic(topic), Some("Sw"));
    }

    #[test]
    fn test_unknown_topic_returns_none() {
        let dict = SymbolDictionary::default();
        let unknown = H256::zero();
        assert_eq!(dict.symbol_for_topic(unknown), None);
    }

    #[test]
    fn test_register_custom() {
        let mut dict = SymbolDictionary::default();
        let custom_topic = H256::repeat_byte(0xAB);
        dict.register_custom(custom_topic, "Custom");
        assert_eq!(dict.symbol_for_topic(custom_topic), Some("Custom"));
    }

    #[test]
    fn test_empty_dictionary() {
        let dict = SymbolDictionary::empty();
        assert!(dict.is_empty());
        assert_eq!(dict.len(), 0);
    }
}
