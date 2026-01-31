use serde_json::Value;
use crate::error::{Result, SodsError};

pub fn migrate_registry(data: &mut Value) -> Result<()> {
    // If version is missing, assume it's a legacy pre-versioned format or v1.0
    let current_version = data.get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("1.0");
    
    match current_version {
        "1.0" => {
            migrate_v1_to_v2(data)?;
            data["version"] = Value::String("2.0".to_string());
        }
        "2.0" => {} // Current version
        _ => return Err(SodsError::ConfigError(format!("Unsupported registry version: {}", current_version))),
    }
    
    Ok(())
}

fn migrate_v1_to_v2(data: &mut Value) -> Result<()> {
    // V1 structure was: { "contracts": { "addr": [deployer, block] }, "last_updated": u64 }
    // V2 structure is:  { "version": "2.0", "contracts": { "addr": { "deployer": addr, "block": block, "name": "..." } }, "last_updated": u64 }
    
    if let Some(contracts) = data.get_mut("contracts").and_then(|c| c.as_object_mut()) {
        for (_addr, entry) in contracts.iter_mut() {
            // Conversion if it's the old [deployer, block] array
            if let Some(arr) = entry.as_array() {
                if arr.len() >= 2 {
                    let deployer = arr[0].clone();
                    let block = arr[1].clone();
                    
                    let mut new_entry = serde_json::Map::new();
                    new_entry.insert("deployer".to_string(), deployer);
                    new_entry.insert("block".to_string(), block);
                    new_entry.insert("name".to_string(), Value::String("Migrated".to_string()));
                    
                    *entry = Value::Object(new_entry);
                }
            } else if entry.is_object() {
                // Already an object, ensure "name" exists for schema compliance
                if entry.get("name").is_none() {
                    entry.as_object_mut().unwrap().insert("name".to_string(), Value::String("Unknown".to_string()));
                }
            }
        }
    }
    
    Ok(())
}
