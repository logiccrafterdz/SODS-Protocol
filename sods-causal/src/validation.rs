//! Validation Registry integration for ERC-8004.
//!
//! This module listens for on-chain validation requests and
//! triggers the causal verification engine.

use serde::{Deserialize, Serialize};
use ethers::types::{H256, Address};
use crate::error::{CausalError, Result};
use crate::proof::CausalBehavioralProof;

/// Represents an incoming validation request from the on-chain registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRequest {
    /// Unique identifier for the request
    pub request_id: H256,
    /// Address of the agent to be validated
    pub agent_id: Address,
    /// Encoded proof data provided by the client
    pub proof_data: Vec<u8>,
    /// Target timestamp for verification
    pub timestamp: u64,
}

/// Represents the response to a validation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub request_id: H256,
    /// Verification score (0 or 100 in current implementation)
    pub score: u32,
    /// Metadata or reason for the score
    pub metadata: String,
}

/// Core logic for handling validation requests.
pub struct ValidationHandler;

impl ValidationHandler {
    /// Validates a behavioral claim using the causal engine.
    pub fn handle_request(request: ValidationRequest) -> Result<ValidationResponse> {
        // 1. Deserialize the proof
        let proof: CausalBehavioralProof = serde_json::from_slice(&request.proof_data)
            .map_err(CausalError::Serialization)?;

        // 2. Perform cryptographic and behavioral verification
        let is_valid = proof.verify(request.timestamp);
        
        // 3. Construct response
        Ok(ValidationResponse {
            request_id: request.request_id,
            score: if is_valid { 100 } else { 0 },
            metadata: if is_valid {
                "Behavioral proof successfully verified by SODS causal engine".to_string()
            } else {
                "Behavioral proof verification failed".to_string()
            },
        })
    }
}
