//! Reputation Registry integration for ERC-8004 trustless agents.
//!
//! This module handles incoming feedback, validates metrics, and
//! prepares reputation claims for on-chain submission.

use serde::{Deserialize, Serialize};
use crate::error::{CausalError, Result};

/// Supported reputation tags for SODS causal verifier.
pub const TAG_ACCURACY: &str = "behavioral_proof_accuracy";
pub const TAG_SPEED: &str = "causal_verification_speed";
pub const TAG_RELIABILITY: &str = "agent_reliability";

/// Represents a single piece of reputation feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationFeedback {
    /// High-level metric category (e.g., "behavioral_proof_accuracy")
    pub tag1: String,
    /// Detailed metric sub-category (optional)
    pub tag2: String,
    /// Numerical value (0-100 or metric specific)
    pub value: u32,
    /// Optional metadata or comment
    pub metadata: Option<String>,
}

impl ReputationFeedback {
    /// Validates the feedback structure according to ERC-8004 requirements.
    pub fn validate(&self) -> Result<()> {
        match self.tag1.as_str() {
            TAG_ACCURACY => {
                if self.value > 100 {
                    return Err(CausalError::InternalError("Accuracy value must be 0-100".to_string()));
                }
            }
            TAG_SPEED => {
                // Speed is in ms, no hard upper limit but shouldn't be zero for a real request
                if self.value == 0 {
                    return Err(CausalError::InternalError("Speed index cannot be zero".to_string()));
                }
            }
            TAG_RELIABILITY => {
                if self.value > 100 {
                    return Err(CausalError::InternalError("Reliability percentage must be 0-100".to_string()));
                }
            }
            _ => return Err(CausalError::InternalError(format!("Unsupported reputation tag: {}", self.tag1))),
        }
        Ok(())
    }

    /// Generates an automatic response message for the feedback.
    pub fn generate_response(&self) -> String {
        format!(
            "SODS auto-reply: Feedback received for {}. Value recorded: {}. Status: Validated.",
            self.tag1, self.value
        )
    }
}

/// Represents a reputation claim to be submitted on-chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationClaim {
    /// Hash of the full feedback stored off-chain (IPFS)
    pub feedback_hash: String,
    /// The feedback data itself
    pub data: ReputationFeedback,
    /// The agent's response
    pub response: String,
}

impl ReputationClaim {
    pub fn new(feedback: ReputationFeedback, ipfs_hash: String) -> Self {
        let response = feedback.generate_response();
        Self {
            feedback_hash: ipfs_hash,
            data: feedback,
            response,
        }
    }
}
