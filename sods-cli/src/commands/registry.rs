use clap::{Args, Subcommand};
use sods_core::ContractRegistry;
use ethers_core::types::Address;
use crate::output;
use std::str::FromStr;

#[derive(Args)]
pub struct RegistryArgs {
    #[command(subcommand)]
    pub command: RegistryCommands,
}

#[derive(Subcommand)]
pub enum RegistryCommands {
    /// Add a contract and its deployer to the registry
    Add {
        /// Contract address
        #[arg(short, long)]
        contract: String,
        /// Deployer address
        #[arg(short, long)]
        deployer: String,
        /// Deployment block number
        #[arg(short, long)]
        block: u64,
    },
    /// Import contracts from a JSON file
    Import {
        /// Path to JSON file
        path: String,
    },
    /// List all known contracts in the registry
    List,
    /// Clear the local registry
    Clear,
}

pub fn run(args: RegistryArgs) -> i32 {
    let mut registry = match ContractRegistry::load_local() {
        Ok(r) => r,
        Err(e) => {
            output::error(&format!("Failed to load registry: {}", e));
            return 1;
        }
    };

    match args.command {
        RegistryCommands::Add { contract, deployer, block } => {
            let contract_addr = match Address::from_str(&contract) {
                Ok(a) => a,
                Err(_) => {
                    output::error(&format!("Invalid contract address: {}", contract));
                    return 1;
                }
            };
            let deployer_addr = match Address::from_str(&deployer) {
                Ok(a) => a,
                Err(_) => {
                    output::error(&format!("Invalid deployer address: {}", deployer));
                    return 1;
                }
            };

            registry.add(contract_addr, deployer_addr, block);
            if let Err(e) = registry.save_local() {
                output::error(&format!("Failed to save registry: {}", e));
                return 1;
            }
            output::success(&format!("Added {} (Deployer: {}) to registry.", contract, deployer));
        }

        RegistryCommands::Import { path } => {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    output::error(&format!("Failed to read file {}: {}", path, e));
                    return 1;
                }
            };

            let new_data: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(e) => {
                    output::error(&format!("Failed to parse JSON: {}", e));
                    return 1;
                }
            };

            // Basic import logic for the format: { "address": { "deployer": "...", "block": ... } }
            if let Some(obj) = new_data.as_object() {
                let mut count = 0;
                for (addr_str, val) in obj {
                    if let (Ok(addr), Some(deployer_str), Some(block)) = (
                        Address::from_str(addr_str),
                        val.get("deployer").and_then(|v| v.as_str()),
                        val.get("block").and_then(|v| v.as_u64())
                    ) {
                        if let Ok(deployer_addr) = Address::from_str(deployer_str) {
                            registry.add(addr, deployer_addr, block);
                            count += 1;
                        }
                    }
                }
                if let Err(e) = registry.save_local() {
                    output::error(&format!("Failed to save registry: {}", e));
                    return 1;
                }
                output::success(&format!("Imported {} contracts from {}.", count, path));
            } else {
                output::error("Invalid JSON format. Expected an object mapping addresses to metadata.");
                return 1;
            }
        }

        RegistryCommands::List => {
            if registry.contracts.is_empty() {
                output::info("Registry is empty.");
            } else {
                output::info(&format!("Registry contains {} entries:", registry.contracts.len()));
                println!("{:<44} | {:<44} | {:<10}", "Contract", "Deployer", "Block");
                println!("{:-<44}-+-{:-<44}-+-{:-<10}", "", "", "");
                for (contract, (deployer, block)) in &registry.contracts {
                    println!("{:<44} | {:<44} | {:<10}", format!("{:?}", contract), format!("{:?}", deployer), block);
                }
            }
        }

        RegistryCommands::Clear => {
            registry.contracts.clear();
            if let Err(e) = registry.save_local() {
                output::error(&format!("Failed to save registry: {}", e));
                return 1;
            }
            output::success("Registry cleared.");
        }
    }

    0
}
