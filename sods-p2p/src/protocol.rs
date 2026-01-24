//! Protocol types for P2P proof exchange with cryptographic signing.

use k256::ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Protocol name for SODS proof exchange.
pub const PROTOCOL_NAME: &str = "/sods/proof/1.0.0";

/// Protocol name for SODS Proof-of-Behavior puzzles.
pub const PUZZLE_PROTOCOL_NAME: &str = "/sods/puzzle/1.0.0";

/// Request for a behavioral proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofRequest {
    /// The symbol to verify (e.g., "Tf", "Dep")
    pub symbol: String,
    /// The block number to query
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PuzzleChallenge {
    pub chain_id: u64,
    pub block_number: u64,
    pub symbol: String,
}

impl PuzzleChallenge {
    pub fn random() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Self {
            chain_id: 11155111, // Sepolia
            block_number: 10002000 + rng.gen_range(0..1000),
            symbol: "Tf".to_string(),
        }
    }
}

/// A solution to a Proof-of-Behavior puzzle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PuzzleSolution {
    pub occurrences: u32,
    /// Success indicator
    pub success: bool,
}

/// Response containing a behavioral proof with cryptographic signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResponse {
    /// Serialized sods_core::Proof bytes
    pub proof_bytes: Vec<u8>,
    /// Behavioral Merkle Root for the block
    pub bmt_root: [u8; 32],
    /// Whether the request was successful
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of symbol occurrences in block
    pub occurrences: usize,
    /// ECDSA signature (64 bytes) over the response hash
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
    /// Compressed public key (33 bytes) of the signer
    #[serde(with = "serde_bytes")]
    pub public_key: Vec<u8>,
}

/// Serde helper for Vec<u8> as bytes
mod serde_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer)
    }
}

impl ProofResponse {
    /// Create a successful signed response.
    pub fn success_signed(
        proof_bytes: Vec<u8>,
        bmt_root: [u8; 32],
        occurrences: usize,
        signing_key: &SigningKey,
    ) -> Self {
        let mut resp = Self {
            proof_bytes,
            bmt_root,
            success: true,
            error: None,
            occurrences,
            signature: Vec::new(),
            public_key: Vec::new(),
        };
        resp.sign(signing_key);
        resp
    }

    /// Create an error response (signed).
    pub fn error_signed(message: impl Into<String>, signing_key: &SigningKey) -> Self {
        let mut resp = Self {
            proof_bytes: Vec::new(),
            bmt_root: [0u8; 32],
            success: false,
            error: Some(message.into()),
            occurrences: 0,
            signature: Vec::new(),
            public_key: Vec::new(),
        };
        resp.sign(signing_key);
        resp
    }

    /// Create a successful response (unsigned, for testing).
    pub fn success(proof_bytes: Vec<u8>, bmt_root: [u8; 32], occurrences: usize) -> Self {
        Self {
            proof_bytes,
            bmt_root,
            success: true,
            error: None,
            occurrences,
            signature: Vec::new(),
            public_key: Vec::new(),
        }
    }

    /// Create an error response (unsigned, for testing).
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            proof_bytes: Vec::new(),
            bmt_root: [0u8; 32],
            success: false,
            error: Some(message.into()),
            occurrences: 0,
            signature: Vec::new(),
            public_key: Vec::new(),
        }
    }

    /// Compute the hash of the signable content.
    fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.proof_bytes);
        hasher.update(&self.bmt_root);
        hasher.update(&[self.success as u8]);
        hasher.update(self.error.as_deref().unwrap_or("").as_bytes());
        hasher.update(&self.occurrences.to_le_bytes());
        hasher.finalize().into()
    }

    /// Sign the response with the given key.
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let hash = self.compute_hash();
        let signature: Signature = signing_key.sign(&hash);
        self.signature = signature.to_bytes().to_vec();
        self.public_key = signing_key.verifying_key().to_sec1_bytes().to_vec();
    }

    /// Verify the response signature.
    pub fn verify_signature(&self) -> bool {
        if self.signature.len() != 64 || self.public_key.is_empty() {
            return false;
        }

        let Ok(sig) = Signature::from_slice(&self.signature) else {
            return false;
        };

        let Ok(pubkey) = VerifyingKey::from_sec1_bytes(&self.public_key) else {
            return false;
        };

        let hash = self.compute_hash();
        pubkey.verify(&hash, &sig).is_ok()
    }

    /// Check if the response is signed.
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty() && !self.public_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::SigningKey;
    use rand::Rng;
    // use rand::rngs::OsRng;

    #[test]
    fn test_request_serialization() {
        let req = ProofRequest {
            symbol: "Dep".to_string(),
            block_number: 10002322,
        };

        let json = serde_json::to_string(&req).unwrap();
        let decoded: ProofRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(decoded.symbol, "Dep");
        assert_eq!(decoded.block_number, 10002322);
    }

    #[test]
    fn test_response_success() {
        let resp = ProofResponse::success(vec![1, 2, 3], [0xAB; 32], 5);
        assert!(resp.success);
        assert!(resp.error.is_none());
        assert_eq!(resp.occurrences, 5);
    }

    #[test]
    fn test_response_error() {
        let resp = ProofResponse::error("Symbol not found");
        assert!(!resp.success);
        assert!(resp.error.is_some());
    }

    #[test]
    fn test_signed_response() {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill(&mut seed);
        let signing_key = SigningKey::from_slice(&seed).unwrap();
        let resp = ProofResponse::success_signed(vec![1, 2, 3], [0xAB; 32], 5, &signing_key);

        assert!(resp.is_signed());
        assert!(resp.verify_signature());
    }

    #[test]
    fn test_tampered_response_fails_verification() {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill(&mut seed);
        let signing_key = SigningKey::from_slice(&seed).unwrap();
        let mut resp = ProofResponse::success_signed(vec![1, 2, 3], [0xAB; 32], 5, &signing_key);

        // Tamper with the response
        resp.occurrences = 100;

        assert!(!resp.verify_signature());
    }

    #[test]
    fn test_wrong_key_fails_verification() {
        let mut seed1 = [0u8; 32];
        let mut seed2 = [0u8; 32];
        rand::thread_rng().fill(&mut seed1);
        rand::thread_rng().fill(&mut seed2);
        let signing_key1 = SigningKey::from_slice(&seed1).unwrap();
        let signing_key2 = SigningKey::from_slice(&seed2).unwrap();

        let mut resp = ProofResponse::success_signed(vec![1, 2, 3], [0xAB; 32], 5, &signing_key1);

        // Replace with different public key
        resp.public_key = signing_key2.verifying_key().to_sec1_bytes().to_vec();

        assert!(!resp.verify_signature());
    }
}
