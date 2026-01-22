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

// ============================================================================
// Core Event Topic Hashes (keccak256 of event signatures)
// ============================================================================

/// ERC20 Transfer(address indexed from, address indexed to, uint256 value)
/// Also ERC721 Transfer(address indexed from, address indexed to, uint256 tokenId)
const TRANSFER_TOPIC: &str = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

/// WETH Deposit(address indexed dst, uint256 wad)
const DEPOSIT_TOPIC: &str = "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c";

/// WETH Withdrawal(address indexed src, uint256 wad)
const WITHDRAWAL_TOPIC: &str = "0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65";

/// Uniswap V2 Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)
const SWAP_V2_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
// Uniswap V3 Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
const SWAP_V3_TOPIC: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

/// Uniswap V2 Mint(address indexed sender, uint256 amount0, uint256 amount1) - Add Liquidity
const MINT_V2_TOPIC: &str = "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26391d14d59cf6ad";

/// Uniswap V2 Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to) - Remove Liquidity
const BURN_V2_TOPIC: &str = "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496";

/// Seaport OrderFulfilled(...)
const SEAPORT_ORDER_FULFILLED: &str = "0x9d9af8e38d66c62e2c12f0225249fd9d721c54b83f48d9352c97c6cacdcb6f31";

/// Optimism DepositFinalized(...) / L1->L2 Bridge
const OPTIMISM_DEPOSIT_FINALIZED: &str = "0xeb2b8427f7a793d5d7107771239e3ec40089856f67566606f35b6279f06574f2";

use ethers_core::types::{Address, U256};

// ============================================================================
// Symbol Dictionary
// ============================================================================

/// Dictionary mapping EVM event topics to behavioral symbol codes.
///
/// The dictionary comes pre-loaded with core immutable symbols.
///
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
            (SWAP_V3_TOPIC, "Sw"),
            (MINT_V2_TOPIC, "LP+"),
            (BURN_V2_TOPIC, "LP-"),
            (SEAPORT_ORDER_FULFILLED, "BuyNFT"),
            (OPTIMISM_DEPOSIT_FINALIZED, "BridgeIn"),
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
    #[inline]
    pub fn symbol_for_topic(&self, topic: H256) -> Option<&'static str> {
        self.registry.get(&topic).copied()
    }

    /// Register a custom symbol for a topic.
    pub fn register_custom(&mut self, topic: H256, symbol: &'static str) {
        self.registry.insert(topic, symbol);
    }

    /// Parse an EVM log into a behavioral symbol.
    pub fn parse_log(&self, log: &Log) -> Option<BehavioralSymbol> {
        // Get topic[0] (event signature hash)
        let topic = log.topics.first()?;

        // Look up symbol
        let mut symbol_code = self.symbol_for_topic(*topic)?; 

        // Extract log index
        let log_index = log.log_index.map(|i| i.as_u32()).unwrap_or(0);

        // --- Context Extraction ---
        let mut from = Address::zero();
        let mut to = Address::zero();
        let mut value = U256::zero();
        let mut token_id = None;
        
        // * Heuristic Extraction Logic based on standard event layouts *
        
        // Handle Transfer (ERC20 / ERC721)
        // Topic0: Sign
        // Topic1: From
        // Topic2: To
        // Data/Topic3: Value or TokenId
        if *topic == TRANSFER_TOPIC.parse::<H256>().unwrap() { // Standard Transfer
             if log.topics.len() >= 3 {
                 from = Address::from(log.topics[1]);
                 to = Address::from(log.topics[2]);
                 
                 // If Topic3 exists, it's likely ERC721 TokenID
                 if log.topics.len() == 4 {
                     token_id = Some(U256::from_big_endian(log.topics[3].as_bytes()));
                     
                     // Helper: Check if Mint (from 0x0)
                     if from == Address::zero() {
                         symbol_code = "MintNFT";
                     }
                 } else {
                     // Check Data for Value (ERC20)
                     if log.data.len() >= 32 {
                         value = U256::from_big_endian(&log.data[0..32]);
                     }
                 }
             }
        } 
        // Handle Swap V2
        else if symbol_code == "Sw" {
            // Context usually from args; keeping simple for now
            // V2: sender is topic 1, to is topic 2 (or 3 depending on indexed)
        }
        
        // Construct Symbol
        Some(BehavioralSymbol::new(symbol_code, log_index)
            .with_context(from, to, value, token_id))
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
    fn test_mint_nft_detection() {
        // Test that Transfer from 0x0 is detected as MintNFT
        let dict = SymbolDictionary::default();
        let topic = TRANSFER_TOPIC.parse::<H256>().unwrap();
        
        let mut log = Log::default();
        log.topics = vec![
            topic, 
            H256::zero(), // from = 0x0
            H256::repeat_byte(0x1), // to = ...
            H256::from_low_u64_be(123) // token_id = 123
        ];
        
        // We need to insert the Transfer mapping first if it wasn't pre-loaded?
        // Ah, default() loads it.
        // Wait, transferring logic is hardcoded inside parse_log to check for Transfer topic specifically
        
        let sym = dict.parse_log(&log).unwrap();
        assert_eq!(sym.symbol(), "MintNFT");
        assert_eq!(sym.token_id, Some(U256::from(123)));
    }

    #[test]
    fn test_seaport_buynft() {
        let dict = SymbolDictionary::default();
        let topic = SEAPORT_ORDER_FULFILLED.parse::<H256>().unwrap();
        assert_eq!(dict.symbol_for_topic(topic), Some("BuyNFT"));
    }

    #[test]
    fn test_default_has_core_symbols() {
        let dict = SymbolDictionary::default();
        // 6 core + 1 v3 + 1 seaport + 1 optimism = 9?
        assert!(dict.len() >= 6); 
    }
    
    // ... existing basic lookups ...
    
    #[test]
    fn test_transfer_lookup() {
        let dict = SymbolDictionary::default();
        let topic = TRANSFER_TOPIC.parse::<H256>().unwrap();
        assert_eq!(dict.symbol_for_topic(topic), Some("Tf"));
    }
}
