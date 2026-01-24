//! Symbol dictionary for mapping EVM event topics to behavioral symbols.
//!
//! This module provides the `SymbolDictionary` which maps known event topic
//! hashes (keccak256 of event signatures) to their corresponding behavioral
//! symbol codes.

use ethers_core::types::{Log, H256};
use std::collections::{HashMap, HashSet};

use crate::symbol::BehavioralSymbol;

// ============================================================================
// Core Event Topic Hashes (keccak256 of event signatures)
// ============================================================================

// ============================================================================
// Core Event Signatures
// ============================================================================

/// Compute topic0 (keccak256 hash) of an event signature.
pub fn event_signature_to_topic0(signature: &str) -> H256 {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(signature.as_bytes());
    H256::from_slice(&hasher.finalize())
}

/// ERC20 Transfer(address indexed from, address indexed to, uint256 value)
const TRANSFER_SIG: &str = "Transfer(address,address,uint256)";

/// WETH Deposit(address indexed dst, uint256 wad)
const DEPOSIT_SIG: &str = "Deposit(address,uint256)";

/// WETH Withdrawal(address indexed src, uint256 wad)
const WITHDRAWAL_SIG: &str = "Withdrawal(address,uint256)";

/// Uniswap V2 Swap(address indexed sender, uint256 amount0In, uint256 amount1In, uint256 amount0Out, uint256 amount1Out, address indexed to)
const SWAP_V2_SIG: &str = "Swap(address,uint256,uint256,uint256,uint256,address)";

/// Uniswap V3 Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
const SWAP_V3_SIG: &str = "Swap(address,address,int256,int256,uint160,uint128,int24)";

/// Uniswap V2 Mint(address indexed sender, uint256 amount0, uint256 amount1)
const MINT_V2_SIG: &str = "Mint(address,uint256,uint256)";

/// Uniswap V2 Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to)
const BURN_V2_SIG: &str = "Burn(address,uint256,uint256,address)";

/// Seaport OrderFulfilled(...)
const SEAPORT_ORDER_FULFILLED_SIG: &str = "OrderFulfilled(bytes32,address,address,address,(uint8,address,uint256,uint256)[],(uint8,address,uint256,uint256,address)[])";

/// Optimism DepositFinalized(...) / L1->L2 Bridge
const OPTIMISM_DEPOSIT_FINALIZED_SIG: &str = "DepositFinalized(address,address,address,address,uint256,bytes)";

/// Blur: OrdersMatched(...)
const BLUR_ORDERS_MATCHED_SIG: &str = "OrdersMatched(address,uint256,bytes32,uint256,address,uint256,uint256,uint256,uint256)";

/// Arbitrum OutboundTransferInitiated
const ARBITRUM_OUTBOUND_TRANSFER_SIG: &str = "OutboundTransferInitiated(address,address,address,uint256,uint256,bytes)";

/// Scroll L2->L1 MessageSent
const SCROLL_MESSAGE_SENT_SIG: &str = "MessageSent(address,address,uint256,uint256,bytes)";

/// Scroll FinalizeDepositERC20
const SCROLL_FINALIZE_DEPOSIT_ERC20_SIG: &str = "FinalizeDepositERC20(address,address,address,address,uint256,bytes)";

/// Scroll WithdrawalInitiated
const SCROLL_WITHDRAWAL_INITIATED_SIG: &str = "WithdrawalInitiated(address,address,address,address,uint256,bytes)";

/// ERC-4337 UserOperationEvent
const AA_OP_SIG: &str = "UserOperationEvent(bytes32,address,address,uint256,bool,uint256,uint256)";

/// Permit2 Permit(...)
const PERMIT2_SIG: &str = "Permit(address,address,uint256,uint256,address,uint256)";

/// CoW Swap Trade(...)
const COW_TRADE_SIG: &str = "Trade(address,address,address,uint256,uint256,uint256,bytes)";

use ethers_core::types::{Address, U256};

// ============================================================================
// Symbol Dictionary
// ============================================================================

/// Dictionary mapping EVM event topics to behavioral symbol codes.
#[derive(Debug, Clone)]
pub struct SymbolDictionary {
    /// Mapping from topic hash to symbol code
    registry: HashMap<H256, &'static str>,
    /// Dynamic plugins (Symbol -> ParserType)
    dynamic_registry: HashMap<H256, String>,
    /// Parser logic map
    plugin_parsers: HashMap<H256, crate::plugins::ParserType>,
}

impl Default for SymbolDictionary {
    /// Create a dictionary with all core immutable symbols.
    fn default() -> Self {
        let mut registry = HashMap::new();

        // Compute and insert core symbols at runtime
        let core_signatures: &[(&str, &'static str)] = &[
            (TRANSFER_SIG, "Tf"),
            (DEPOSIT_SIG, "Dep"),
            (WITHDRAWAL_SIG, "Wdw"),
            (SWAP_V2_SIG, "Sw"),
            (SWAP_V3_SIG, "Sw"),
            (MINT_V2_SIG, "LP+"),
            (BURN_V2_SIG, "LP-"),
            (SEAPORT_ORDER_FULFILLED_SIG, "BuyNFT"),
            (BLUR_ORDERS_MATCHED_SIG, "ListNFT"),
            (OPTIMISM_DEPOSIT_FINALIZED_SIG, "BridgeIn"),
            (ARBITRUM_OUTBOUND_TRANSFER_SIG, "BridgeOut"),
            (SCROLL_MESSAGE_SENT_SIG, "BridgeOut"),
            (SCROLL_FINALIZE_DEPOSIT_ERC20_SIG, "BridgeIn"),
            (SCROLL_WITHDRAWAL_INITIATED_SIG, "BridgeOut"),
            (AA_OP_SIG, "AAOp"),
            (PERMIT2_SIG, "Permit2"),
            (COW_TRADE_SIG, "CoWTrade"),
        ];

        for (sig, symbol) in core_signatures {
            let topic = event_signature_to_topic0(sig);
            registry.insert(topic, *symbol);
        }

        Self { 
            registry,
            dynamic_registry: HashMap::new(),
            plugin_parsers: HashMap::new(),
        }
    }
}

impl SymbolDictionary {
    /// Create an empty dictionary (no pre-loaded symbols).
    pub fn empty() -> Self {
        Self {
            registry: HashMap::new(),
            dynamic_registry: HashMap::new(),
            plugin_parsers: HashMap::new(),
        }
    }

    /// Look up the symbol for a given event topic.
    #[inline]
    pub fn symbol_for_topic(&self, topic: H256) -> Option<&str> {
        if let Some(s) = self.registry.get(&topic) {
            return Some(s);
        }
        self.dynamic_registry.get(&topic).map(|s| s.as_str())
    }

    /// Look up all event topics associated with a symbol code.
    pub fn topics_for_symbol(&self, symbol: &str) -> Vec<H256> {
        let mut topics = Vec::new();
        
        // Search static registry
        for (topic, code) in &self.registry {
            if *code == symbol {
                topics.push(*topic);
            }
        }

        // Search dynamic registry
        for (topic, code) in &self.dynamic_registry {
            if code == symbol {
                topics.push(*topic);
            }
        }

        topics
    }

    /// Map a behavioral pattern to the set of required Ethereum topic hashes.
    pub fn pattern_to_required_topics(&self, pattern: &crate::pattern::BehavioralPattern) -> Vec<H256> {
        use crate::pattern::PatternStep;
        let mut required_topics = HashSet::new(); // Use HashSet to avoid duplicates

        for step in pattern.steps() {
            let symbol_code = match step {
                PatternStep::Exact(s, _) => s,
                PatternStep::AtLeast(s, _, _) => s,
                PatternStep::Range(s, _, _, _) => s,
            };
            
            for topic in self.topics_for_symbol(symbol_code) {
                required_topics.insert(topic);
            }
        }

        required_topics.into_iter().collect()
    }

    /// Register a custom symbol for a topic.
    pub fn register_custom(&mut self, topic: H256, symbol: &'static str) {
        self.registry.insert(topic, symbol);
    }

    /// Register a dynamic plugin based symbol.
    pub fn register_plugin(&mut self, plugin: crate::plugins::SymbolPlugin) {
        self.dynamic_registry.insert(plugin.event_topic, plugin.symbol);
        self.plugin_parsers.insert(plugin.event_topic, plugin.parser);
    }

    /// Parse an EVM log into a behavioral symbol.
    pub fn parse_log(&self, log: &Log) -> Option<BehavioralSymbol> {
        // Get topic[0] (event signature hash)
        let topic = log.topics.first()?;

        // Look up symbol
        let symbol_code = self.symbol_for_topic(*topic)?; 

        // Extract log index
        let log_index = log.log_index.map(|i| i.as_u32()).unwrap_or(0);

        // --- Context Extraction ---
        let mut from = Address::zero();
        let mut to = Address::zero();
        let mut value = U256::zero();
        let mut token_id = None;
        
        // Check for specific parser override
        if let Some(parser) = self.plugin_parsers.get(topic) {
            match parser {
                crate::plugins::ParserType::Generic => {
                    // Just basic symbol, no context extracted yet
                },
                crate::plugins::ParserType::Transfer => {
                     // Generic Transfer Logic (Assuming standard topics 1=from, 2=to)
                     if log.topics.len() >= 3 {
                         from = Address::from(log.topics[1]);
                         to = Address::from(log.topics[2]);
                     }
                },
                crate::plugins::ParserType::Swap => {
                    // Generic Swap (Assuming sender is first indexed topic)
                    if log.topics.len() >= 2 {
                        from = Address::from(log.topics[1]);
                    }
                }
            }
        } 
        // Fallback to legacy hardcoded heuristic if no plugin or plugin is generic
        else if *topic == event_signature_to_topic0(TRANSFER_SIG) { // Standard Transfer
             if log.topics.len() >= 3 {
                 from = Address::from(log.topics[1]);
                 to = Address::from(log.topics[2]);
                 
                 // If Topic3 exists, it's likely ERC721 TokenID
                 if log.topics.len() == 4 {
                     token_id = Some(U256::from_big_endian(log.topics[3].as_bytes()));
                     
                     // Helper: Check if Mint (from 0x0)
                     if from == Address::zero() && symbol_code == "Tf" { // Only override if it's the base code
                         return Some(BehavioralSymbol::new("MintNFT", log_index)
                            .with_context(from, to, value, token_id));
                     }
                 } else if log.data.len() >= 32 {
                     value = U256::from_big_endian(&log.data[0..32]);
                 }
             }
        } 
        else if *topic == event_signature_to_topic0(AA_OP_SIG) { // ERC-4337
            if log.topics.len() >= 3 {
                let op_hash = log.topics[1];
                let sender = Address::from(log.topics[2]);
                
                return Some(BehavioralSymbol::new("AAOp", log_index)
                    .with_context(sender, Address::zero(), U256::zero(), None)
                    .with_aa_context(op_hash));
            }
        }
        else if *topic == event_signature_to_topic0(PERMIT2_SIG) { // Permit2
            if log.topics.len() >= 3 {
                let owner = Address::from(log.topics[1]);
                let spender = Address::from(log.topics[2]);
                
                // deadline is usually in data, let's assume standard layout
                // index 32: value, 64: expiration, 96: nonce, 128: signature...
                let mut deadline = 0;
                if log.data.len() >= 64 {
                    deadline = U256::from_big_endian(&log.data[32..64]).as_u64();
                    value = U256::from_big_endian(&log.data[0..32]);
                }

                return Some(BehavioralSymbol::new("Permit2", log_index)
                    .with_context(owner, spender, value, None)
                    .with_permit2_context(deadline));
            }
        }
        else if *topic == event_signature_to_topic0(COW_TRADE_SIG) { // CoW Swap
            if log.topics.len() >= 2 {
                let owner = Address::from(log.topics[1]);
                
                // GPv2 Settlement Trade(owner, sellToken, buyToken, sellAmount, buyAmount, feeAmount, orderUid)
                // Topics[1] is owner. Data contains tokens and amounts.
                if log.data.len() >= 128 {
                    // let sell_token = Address::from(H256::from_slice(&log.data[12..32]));
                    to = Address::from(H256::from_slice(&log.data[44..64])); // buyToken
                    value = U256::from_big_endian(&log.data[96..128]); // buyAmount
                }

                return Some(BehavioralSymbol::new("CoWTrade", log_index)
                    .with_context(owner, to, value, None));
            }
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
    fn test_signature_hashing() {
        let topic = event_signature_to_topic0("Transfer(address,address,uint256)");
        let expected = "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
        assert_eq!(hex::encode(topic), expected);
    }

    #[test]
    fn test_mint_nft_detection() {
        // Test that Transfer from 0x0 is detected as MintNFT
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(TRANSFER_SIG);
        
        let mut log = Log::default();
        log.topics = vec![
            topic, 
            H256::zero(), // from = 0x0
            H256::repeat_byte(0x1), // to = ...
            H256::from_low_u64_be(123) // token_id = 123
        ];
        
        let sym = dict.parse_log(&log).unwrap();
        assert_eq!(sym.symbol(), "MintNFT");
        assert_eq!(sym.token_id, Some(U256::from(123)));
    }

    #[test]
    fn test_seaport_buynft() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(SEAPORT_ORDER_FULFILLED_SIG);
        assert_eq!(dict.symbol_for_topic(topic), Some("BuyNFT"));
    }

    #[test]
    fn test_default_has_core_symbols() {
        let dict = SymbolDictionary::default();
        assert_eq!(dict.len(), 17); 
    }
    
    #[test]
    fn test_transfer_lookup() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(TRANSFER_SIG);
        assert_eq!(dict.symbol_for_topic(topic), Some("Tf"));
    }

    #[test]
    fn test_blur_listnft() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(BLUR_ORDERS_MATCHED_SIG);
        assert_eq!(dict.symbol_for_topic(topic), Some("ListNFT"));
    }

    #[test]
    fn test_optimism_bridge_in() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(OPTIMISM_DEPOSIT_FINALIZED_SIG);
        assert_eq!(dict.symbol_for_topic(topic), Some("BridgeIn"));
    }

    #[test]
    fn test_arbitrum_bridge_out() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(ARBITRUM_OUTBOUND_TRANSFER_SIG);
        assert_eq!(dict.symbol_for_topic(topic), Some("BridgeOut"));
    }

    #[test]
    fn test_scroll_bridge_out() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(SCROLL_MESSAGE_SENT_SIG);
        assert_eq!(dict.symbol_for_topic(topic), Some("BridgeOut"));
    }

    #[test]
    fn test_all_new_symbols_registered() {
        let dict = SymbolDictionary::default();
        assert_eq!(dict.len(), 17);
    }

    #[test]
    fn test_scroll_bridge_in_v2() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(SCROLL_FINALIZE_DEPOSIT_ERC20_SIG);
        assert_eq!(dict.symbol_for_topic(topic), Some("BridgeIn"));
    }

    #[test]
    fn test_scroll_bridge_out_v2() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(SCROLL_WITHDRAWAL_INITIATED_SIG);
        assert_eq!(dict.symbol_for_topic(topic), Some("BridgeOut"));
    }

    #[test]
    fn test_aa_op_parsing() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(AA_OP_SIG);
        let op_hash = H256::repeat_byte(0xAA);
        let sender = Address::repeat_byte(0x11);
        
        let mut log = Log::default();
        log.topics = vec![topic, op_hash, H256::from(sender)];
        log.log_index = Some(10.into());
        
        let sym = dict.parse_log(&log).unwrap();
        assert_eq!(sym.symbol(), "AAOp");
        assert_eq!(sym.user_op_hash, Some(op_hash));
        assert_eq!(sym.from, sender);
    }

    #[test]
    fn test_permit2_parsing() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(PERMIT2_SIG);
        let owner = Address::repeat_byte(0x22);
        let spender = Address::repeat_byte(0x33);
        
        let mut log = Log::default();
        log.topics = vec![topic, H256::from(owner), H256::from(spender)];
        
        // Data: value (32), deadline (32)
        let mut data = vec![0u8; 64];
        let value = U256::from(1000);
        let deadline = 123456789u64;
        value.to_big_endian(&mut data[0..32]);
        U256::from(deadline).to_big_endian(&mut data[32..64]);
        log.data = data.into();
        
        let sym = dict.parse_log(&log).unwrap();
        assert_eq!(sym.symbol(), "Permit2");
        assert_eq!(sym.from, owner);
        assert_eq!(sym.to, spender);
        assert_eq!(sym.value, value);
        assert_eq!(sym.permit_deadline, Some(deadline));
    }

    #[test]
    fn test_cow_trade_parsing() {
        let dict = SymbolDictionary::default();
        let topic = event_signature_to_topic0(COW_TRADE_SIG);
        let owner = Address::repeat_byte(0x44);
        
        let mut log = Log::default();
        log.topics = vec![topic, H256::from(owner)];
        
        // GPv2 Settlement Trade(owner, sellToken, buyToken, sellAmount, buyAmount, feeAmount, orderUid)
        // Data layout (non-indexed): sellToken, buyToken, sellAmount, buyAmount, feeAmount, orderUid
        let mut data = vec![0u8; 160];
        let buy_token = Address::repeat_byte(0x55);
        let buy_amount = U256::from(5000);
        H256::from(buy_token).as_bytes().iter().enumerate().for_each(|(i, &b)| data[32+i] = b); // index 32-64
        buy_amount.to_big_endian(&mut data[96..128]); // buyAmount
        log.data = data.into();
        
        let sym = dict.parse_log(&log).unwrap();
        assert_eq!(sym.symbol(), "CoWTrade");
        assert_eq!(sym.from, owner);
        assert_eq!(sym.to, buy_token);
        assert_eq!(sym.value, buy_amount);
    }
}
