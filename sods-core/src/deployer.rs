use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use ethers_core::types::Address;
use serde::{Deserialize, Serialize};
use crate::error::{Result, SodsError};

/// A registry of known contract deployers.
/// 
/// This local database replaces expensive runtime blockchain searches
/// with a reliable, updatable mapping of contracts to their creators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRegistry {
    /// Map of contract address -> (deployer_address, deployment_block)
    pub contracts: HashMap<Address, (Address, u64)>,
    /// Last verification/sync timestamp
    pub last_updated: u64,
}

impl Default for ContractRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        let mut contracts = HashMap::new();
        
        // Default entry: Uniswap V2 Router (Mainnet)
        if let Ok(addr) = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d".parse() {
            if let Ok(deployer) = "0x8c8d7c46219d9205f05612f8cc93e7c7a6fc2ea5".parse() {
                contracts.insert(addr, (deployer, 9997110));
            }
        }

        Self {
            contracts,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Load the registry from the default local file path (~/.sods/contract_registry.json).
    pub fn load_local() -> Result<Self> {
        let path = Self::get_default_path()?;
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| SodsError::ConfigError(format!("Failed to read registry: {}", e)))?;
        
        serde_json::from_str(&content)
            .map_err(|e| SodsError::ConfigError(format!("Failed to parse registry: {}", e)))
    }

    /// Save the registry to the default local file path.
    pub fn save_local(&self) -> Result<()> {
        let path = Self::get_default_path()?;
        
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| SodsError::ConfigError(format!("Failed to create config dir: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| SodsError::ConfigError(format!("Failed to serialize registry: {}", e)))?;
        
        fs::write(path, content)
            .map_err(|e| SodsError::ConfigError(format!("Failed to write registry: {}", e)))
    }

    /// Add or update a contract in the registry.
    pub fn add(&mut self, contract: Address, deployer: Address, block: u64) {
        self.contracts.insert(contract, (deployer, block));
        self.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get the deployer for a specific contract.
    pub fn get_deployer(&self, contract: &Address) -> Option<Address> {
        self.contracts.get(contract).map(|(deployer, _)| *deployer)
    }

    /// Get the default path for the registry file.
    fn get_default_path() -> Result<PathBuf> {
        home::home_dir()
            .map(|h| h.join(".sods").join("contract_registry.json"))
            .ok_or_else(|| SodsError::ConfigError("Could not determine home directory".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_registry_add_get() {
        let mut reg = ContractRegistry::new();
        let contract = Address::from_str("0x7a250d5630b4cf539739df2c5dacb4c659f2488d").unwrap();
        let deployer = Address::from_str("0x8c8d7c46219d9205f05612f8cc93e7c7a6fc2ea5").unwrap();
        
        reg.add(contract, deployer, 9997110);
        assert_eq!(reg.get_deployer(&contract), Some(deployer));
    }
}
