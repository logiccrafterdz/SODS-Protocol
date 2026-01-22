use serde::{Deserialize, Serialize};
use ethers_core::types::H256;
use crate::error::{Result, SodsError};

/// Type of parser logic to apply for this symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ParserType {
    /// Standard ERC20/721 Transfer logic
    Transfer,
    /// Uniswap V2/V3 Swap logic (Amount0/1)
    Swap,
    /// Generic log (just checks topic presence)
    Generic,
}

/// A dynamic symbol plugin definition loaded from JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolPlugin {
    /// Human readable name (e.g. "Uniswap V3 Swap")
    pub name: String,
    /// The behavioral symbol code (e.g. "SwV3")
    pub symbol: String,
    /// The chain this applies to (optional filter)
    pub chain: Option<String>,
    /// The Keccak256 event topic hash
    pub event_topic: H256,
    /// The parser logic to use
    pub parser: ParserType,
}

impl SymbolPlugin {
    /// Load a plugin from a JSON string.
    pub fn load_from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json)
            .map_err(|e| SodsError::Serialization(format!("Invalid plugin JSON: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_deserialize_plugin() {
        let json = r#"{
            "name": "Uniswap V3 Swap",
            "symbol": "SwV3",
            "chain": "ethereum",
            "event_topic": "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67",
            "parser": "swap"
        }"#;

        let plugin = SymbolPlugin::load_from_json(json).unwrap();
        assert_eq!(plugin.symbol, "SwV3");
        assert_eq!(plugin.parser, ParserType::Swap);
        assert_eq!(plugin.event_topic, H256::from_str("0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67").unwrap());
    }
}
