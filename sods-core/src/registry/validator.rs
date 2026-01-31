use serde_json::Value;
use jsonschema::JSONSchema;
use crate::error::{Result, SodsError};

pub struct RegistryValidator {
    schema: JSONSchema,
}

impl RegistryValidator {
    pub fn new() -> Result<Self> {
        let schema_str = include_str!("schema.json");
        let schema_value: Value = serde_json::from_str(schema_str)
            .map_err(|e| SodsError::ConfigError(format!("Failed to parse internal schema: {}", e)))?;
        
        let schema = JSONSchema::compile(&schema_value)
            .map_err(|e| SodsError::ConfigError(format!("Failed to compile registry schema: {}", e)))?;
            
        Ok(Self { schema })
    }
    
    pub fn validate(&self, registry_data: &Value) -> Result<()> {
        if let Err(mut errors) = self.schema.validate(registry_data) {
            if let Some(first_error) = errors.next() {
                // Collect all error messages
                let mut messages = vec![first_error.to_string()];
                for err in errors {
                    messages.push(err.to_string());
                }
                
                return Err(SodsError::ConfigError(format!(
                    "Contract registry validation failed:\n  - {}", 
                    messages.join("\n  - ")
                )));
            }
        }
        Ok(())
    }
}
