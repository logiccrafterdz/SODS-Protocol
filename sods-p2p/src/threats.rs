//! Decentralized Threat Intelligence types.
//!
//! Defines the structure for threat rules exchanged over P2P gossipsub.

use k256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use ethers_core::types::Address;
use sods_core::pattern::BehavioralPattern;

/// Topic for threat intelligence gossip.
pub const THREATS_TOPIC: &str = "/sods/threats/1.0.0";

/// A behavioral threat rule signed by a researcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatRule {
    /// Unique ID of the rule (e.g. "base-rug-v1")
    pub id: String,
    /// Human readable name
    pub name: String,
    /// Behavioral pattern definition (e.g. "LP+ -> Sw -> LP-")
    pub pattern: String,
    /// Chain identifier (e.g. "base", "ethereum")
    pub chain: String,
    /// Severity level (low, medium, high, critical)
    pub severity: String,
    /// Timestamp of creation (seconds since epoch)
    pub timestamp: u64,
    /// ECDSA signature (64 bytes)
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// Public key of the author (33 bytes compressed)
    #[serde(with = "serde_bytes")]
    pub author_pubkey: Vec<u8>,
}

/// A contract deployer entry in a registry update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEntry {
    pub address: Address,
    pub deployer: Address,
    pub block: u64,
}

/// A collection of contract registry updates signed by a researcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryUpdate {
    /// List of contracts to add/update
    pub contracts: Vec<ContractEntry>,
    /// Timestamp of creation
    pub timestamp: u64,
    /// ECDSA signature (64 bytes)
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// Public key of the author
    #[serde(with = "serde_bytes")]
    pub author_pubkey: Vec<u8>,
}

impl RegistryUpdate {
    pub fn new(contracts: Vec<ContractEntry>, signing_key: &SigningKey) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut update = Self {
            contracts,
            timestamp,
            signature: Vec::new(),
            author_pubkey: signing_key.verifying_key().to_sec1_bytes().to_vec(),
        };
        update.sign(signing_key);
        update
    }

    fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for entry in &self.contracts {
            hasher.update(entry.address.as_bytes());
            hasher.update(entry.deployer.as_bytes());
            hasher.update(&entry.block.to_le_bytes());
        }
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.finalize().into()
    }

    pub fn sign(&mut self, signing_key: &SigningKey) {
        let hash = self.compute_hash();
        let signature: Signature = signing_key.sign(&hash);
        self.signature = signature.to_bytes().to_vec();
    }

    pub fn verify(&self) -> bool {
        if self.signature.len() != 64 || self.author_pubkey.is_empty() {
            return false;
        }
        let Ok(sig) = Signature::from_slice(&self.signature) else { return false; };
        let Ok(pubkey) = VerifyingKey::from_sec1_bytes(&self.author_pubkey) else { return false; };
        
        let hash = self.compute_hash();
        pubkey.verify(&hash, &sig).is_ok()
    }
}

mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer { bytes.serialize(serializer) }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where D: Deserializer<'de> { Vec::<u8>::deserialize(deserializer) }
}

impl ThreatRule {
    /// Create a new signed rule.
    pub fn new(
        id: &str,
        name: &str,
        pattern: &str,
        chain: &str,
        severity: &str,
        signing_key: &SigningKey
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut rule = Self {
            id: id.to_string(),
            name: name.to_string(),
            pattern: pattern.to_string(),
            chain: chain.to_string(),
            severity: severity.to_string(),
            timestamp,
            signature: Vec::new(),
            author_pubkey: signing_key.verifying_key().to_sec1_bytes().to_vec(),
        };
        rule.sign(signing_key);
        rule
    }

    /// Compute hash of the rule content.
    fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.name.as_bytes());
        hasher.update(self.pattern.as_bytes());
        hasher.update(self.chain.as_bytes());
        hasher.update(self.severity.as_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.finalize().into()
    }

    /// Sign the rule.
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let hash = self.compute_hash();
        let signature: Signature = signing_key.sign(&hash);
        self.signature = signature.to_bytes().to_vec();
    }

    /// Verify the signature and pattern syntax.
    pub fn verify(&self) -> bool {
        // 1. Syntax check
        if BehavioralPattern::parse(&self.pattern).is_err() {
            return false;
        }

        // 2. Crypto check
        if self.signature.len() != 64 || self.author_pubkey.is_empty() {
            return false;
        }
        let Ok(sig) = Signature::from_slice(&self.signature) else { return false; };
        let Ok(pubkey) = VerifyingKey::from_sec1_bytes(&self.author_pubkey) else { return false; };
        
        let hash = self.compute_hash();
        pubkey.verify(&hash, &sig).is_ok()
    }
}

/// Registry of validated threat rules.
#[derive(Debug, Default)]
pub struct ThreatRegistry {
    rules: HashMap<String, ThreatRule>,
    trusted_keys: Vec<Vec<u8>>,
    /// Local contract registry to merge updates into
    contract_registry: sods_core::deployer::ContractRegistry,
}

impl ThreatRegistry {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            trusted_keys: Vec::new(),
            contract_registry: sods_core::deployer::ContractRegistry::load_local()
                .unwrap_or_else(|_| sods_core::deployer::ContractRegistry::new()),
        }
    }

    pub fn add_trusted_key(&mut self, key_bytes: Vec<u8>) {
        self.trusted_keys.push(key_bytes);
    }

    /// Add a rule if it is valid and from a trusted author.
    pub fn add_rule(&mut self, rule: ThreatRule) -> bool {
        if !self.trusted_keys.is_empty() && !self.trusted_keys.contains(&rule.author_pubkey) {
            return false;
        }
        if !rule.verify() {
            return false;
        }
        self.rules.insert(rule.id.clone(), rule);
        true
    }

    /// Process a registry update and merge it into the local contract registry.
    pub fn process_registry_update(&mut self, update: RegistryUpdate) -> bool {
        if !self.trusted_keys.is_empty() && !self.trusted_keys.contains(&update.author_pubkey) {
            return false;
        }
        if !update.verify() {
            return false;
        }
        for entry in update.contracts {
            self.contract_registry.add(entry.address, entry.deployer, entry.block, None);
        }
        let _ = self.contract_registry.save_local();
        true
    }

    pub fn get_rules(&self) -> Vec<&ThreatRule> {
        self.rules.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use rand::Rng;
    // use rand::rngs::OsRng;

    #[test]
    fn test_valid_rule_verification() {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill(&mut seed);
        let signing_key = SigningKey::from_slice(&seed).unwrap();
        let rule = ThreatRule::new(
            "test-rule-1",
            "Test Rule",
            "Tf",
            "base",
            "high",
            &signing_key
        );

        assert!(rule.verify());
    }

    #[test]
    fn test_tampered_rule_fails() {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill(&mut seed);
        let signing_key = SigningKey::from_slice(&seed).unwrap();
        let mut rule = ThreatRule::new(
            "test-rule-2",
            "Test Rule",
            "Tf",
            "base",
            "high",
            &signing_key
        );

        // Tamper with severity
        rule.severity = "low".to_string();
        assert!(!rule.verify());
    }

    #[test]
    fn test_invalid_pattern_fails() {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill(&mut seed);
        let signing_key = SigningKey::from_slice(&seed).unwrap();
        let rule = ThreatRule::new(
            "test-rule-3",
            "Test Rule",
            "InvalidPattern{", // Syntax error
            "base",
            "high",
            &signing_key
        );

        // Verification checks both syntax and signature. 
        // Signature might be valid (signed invalid pattern), but verify() checks parse().
        
        assert!(!rule.verify());
    }
}
