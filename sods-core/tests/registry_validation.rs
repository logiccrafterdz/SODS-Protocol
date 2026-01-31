use sods_core::registry::ContractRegistry;
use serde_json::json;
use std::fs;

#[test]
fn test_valid_registry_loads_successfully() {
    let registry = ContractRegistry::new();
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("registry.json");
    
    let content = serde_json::to_string_pretty(&registry).unwrap();
    fs::write(&path, content).unwrap();
    
    // We can't use load_local directly because it uses hardcoded home path
    // So we'll test the internal logic or create a helper
    let mut json_data: serde_json::Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    sods_core::registry::migration::migrate_registry(&mut json_data).unwrap();
    
    let validator = sods_core::registry::validator::RegistryValidator::new().unwrap();
    validator.validate(&json_data).expect("Should validate");
}

#[test]
fn test_v1_to_v2_migration_works() {
    // V1 legacy format (array-based entries)
    let v1_json = json!({
        "contracts": {
            "0x7a250d5630b4cf539739df2c5dacb4c659f2488d": [
                "0x8c8d7c46219d9205f05612f8cc93e7c7a6fc2ea5",
                9997110
            ]
        },
        "last_updated": 123456789
    });

    let mut data = v1_json.clone();
    sods_core::registry::migration::migrate_registry(&mut data).expect("Migration should succeed");

    assert_eq!(data["version"], "2.0");
    assert_eq!(data["contracts"]["0x7a250d5630b4cf539739df2c5dacb4c659f2488d"]["deployer"], "0x8c8d7c46219d9205f05612f8cc93e7c7a6fc2ea5");
    assert_eq!(data["contracts"]["0x7a250d5630b4cf539739df2c5dacb4c659f2488d"]["block"], 9997110);
    assert_eq!(data["contracts"]["0x7a250d5630b4cf539739df2c5dacb4c659f2488d"]["name"], "Migrated");
}

#[test]
fn test_invalid_contract_address_rejected() {
    let validator = sods_core::registry::validator::RegistryValidator::new().unwrap();
    
    let invalid_json = json!({
        "version": "2.0",
        "contracts": {
            "0xinvalid": {
                "deployer": "0x8c8d7c46219d9205f05612f8cc93e7c7a6fc2ea5",
                "block": 100
            }
        },
        "last_updated": 0
    });

    let result = validator.validate(&invalid_json);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Additional properties") || err_msg.contains("pattern"));
}

#[test]
fn test_missing_required_fields_rejected() {
    let validator = sods_core::registry::validator::RegistryValidator::new().unwrap();
    
    let missing_fields = json!({
        "version": "2.0",
        "contracts": {
            "0x7a250d5630b4cf539739df2c5dacb4c659f2488d": {
                "block": 100
                // Missing deployer
            }
        },
        "last_updated": 0
    });

    let result = validator.validate(&missing_fields);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("deployer"));
}

#[test]
fn test_unsupported_version_handled_gracefully() {
    let mut future_version = json!({
        "version": "99.0",
        "contracts": {},
        "last_updated": 0
    });

    let result = sods_core::registry::migration::migrate_registry(&mut future_version);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported registry version"));
}
