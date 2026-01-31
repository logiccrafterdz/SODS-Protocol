pub mod validator;
pub mod migration;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use ethers_core::types::Address;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::{Result, SodsError};
use validator::RegistryValidator;
use migration::migrate_registry;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractEntry {
    pub deployer: Address,
    pub block: u64,
    pub name: String,
}

/// A registry of known contract deployers with versioned schema validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRegistry {
    pub version: String,
    /// Map of contract address -> entry
    pub contracts: HashMap<Address, ContractEntry>,
    /// Last verification/sync timestamp
    pub last_updated: u64,
}

impl Default for ContractRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractRegistry {
    /// Create a new empty registry with the current version (2.0).
    pub fn new() -> Self {
        let mut contracts = HashMap::new();
        
        // Default entry: Uniswap V2 Router (Mainnet)
        if let Ok(addr) = "0x7a250d5630b4cf539739df2c5dacb4c659f2488d".parse() {
            if let Ok(deployer) = "0x8c8d7c46219d9205f05612f8cc93e7c7a6fc2ea5".parse() {
                contracts.insert(addr, ContractEntry {
                    deployer,
                    block: 9997110,
                    name: "Uniswap V2 Router".to_string(),
                });
            }
        }

        Self {
            version: "2.0".to_string(),
            contracts,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Load the registry from the default local file path with validation and migration.
    pub fn load_local() -> Result<Self> {
        let path = Self::get_default_path()?;
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| SodsError::ConfigError(format!("Failed to read registry: {}", e)))?;
        
        let mut json_data: Value = serde_json::from_str(&content)
            .map_err(|e| SodsError::ConfigError(format!("Failed to parse registry JSON: {}", e)))?;

        // 1. Migrate if necessary
        migrate_registry(&mut json_data)?;

        // 2. Validate against schema
        let validator = RegistryValidator::new()?;
        validator.validate(&json_data)?;

        // 3. Deserialize into typed structure
        let registry: Self = serde_json::from_value(json_data)
            .map_err(|e| SodsError::ConfigError(format!("Failed to deserialize registry: {}", e)))?;

        Ok(registry)
    }

    /// Save the registry to the default local file path.
    pub fn save_local(&self) -> Result<()> {
        let path = Self::get_default_path()?;
        
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
    pub fn add(&mut self, contract: Address, deployer: Address, block: u64, name: Option<String>) {
        self.contracts.insert(contract, ContractEntry {
            deployer,
            block,
            name: name.unwrap_or_else(|| "Unknown".to_string()),
        });
        self.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get the deployer for a specific contract.
    pub fn get_deployer(&self, contract: &Address) -> Option<Address> {
        self.contracts.get(contract).map(|entry| entry.deployer)
    }

    /// Get the default path for the registry file.
    pub fn get_default_path() -> Result<PathBuf> {
        home::home_dir()
            .map(|h| h.join(".sods").join("contract_registry.json"))
            .ok_or_else(|| SodsError::ConfigError("Could not determine home directory".into()))
    }
}
