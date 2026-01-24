use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct BootstrapperList {
    pub version: u32,
    pub timestamp: u64,
    pub peers: Vec<String>, // Multiaddr or PeerId strings
    pub signature: String,  // Hex-encoded Ed25519 signature
}

pub struct BootstrapperRegistry {
    sources: Vec<String>,
    verifying_key: VerifyingKey,
}

impl BootstrapperRegistry {
    pub fn new(sources: Vec<String>, public_key_hex: &str) -> Result<Self> {
        let public_key_bytes = hex::decode(public_key_hex)?;
        let verifying_key = VerifyingKey::from_bytes(
            public_key_bytes.as_slice().try_into().map_err(|_| anyhow!("Invalid public key length"))?
        )?;

        Ok(Self {
            sources,
            verifying_key,
        })
    }

    pub async fn fetch_trusted_peers(&self) -> Result<Vec<PeerId>> {
        let mut all_peers = Vec::new();

        for source in &self.sources {
            let response = reqwest::get(source).await?;
            if !response.status().is_success() {
                continue;
            }

            let list: BootstrapperList = response.json().await?;
            
            // Verify signature
            if self.verify_list(&list).is_ok() {
                for peer_str in list.peers {
                    if let Ok(peer_id) = PeerId::from_str(&peer_str) {
                        all_peers.push(peer_id);
                    }
                }
            }
        }

        if all_peers.is_empty() {
            return Err(anyhow!("No trusted peers found or all lists failed verification"));
        }

        Ok(all_peers)
    }

    fn verify_list(&self, list: &BootstrapperList) -> Result<()> {
        // Prepare signed data (excluding signature field)
        // For simplicity in this implementation, we re-serialize or structured hash
        // In a real implementation, we'd sign the JSON bytes before the signature field
        let signature_bytes = hex::decode(&list.signature)?;
        let signature = Signature::from_bytes(
            signature_bytes.as_slice().try_into().map_err(|_| anyhow!("Invalid signature length"))?
        );

        // Simple verification logic (this should ideally be more robust)
        let message = format!("{}:{}:{}", list.version, list.timestamp, list.peers.join(","));
        
        self.verifying_key.verify(message.as_bytes(), &signature)
            .map_err(|e| anyhow!("Signature verification failed: {}", e))
    }
}
