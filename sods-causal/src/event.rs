//! Causal event definition for SODS Agent Registry.
//!
//! This module defines `CausalEvent`, the atomic unit of agent behavior
//! that enables verifiable behavioral interactions without centralized
//! reputation systems.
//!
//! # Example
//!
//! ```rust
//! use sods_causal::CausalEvent;
//! use ethers::types::Address;
//!
//! // Create a causal event for a task execution
//! let event = CausalEvent::builder()
//!     .agent_id("0x1234567890123456789012345678901234567890".parse().unwrap())
//!     .nonce(1)
//!     .sequence_index(0)
//!     .event_type("task_executed")
//!     .result("success")
//!     .timestamp(1700000000)
//!     .build()
//!     .unwrap();
//!
//! assert_eq!(event.result, "success");
//! ```

use ethers::types::{Address, H256};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use crate::error::{CausalError, Result};

/// Valid result values for causal events.
pub const VALID_RESULTS: &[&str] = &["success", "failure", "partial", "timeout"];

/// Atomic unit of agent behavior in the causal event model.
///
/// Each `CausalEvent` represents a single verifiable action performed
/// by an AI agent. Events are ordered causally by `(nonce, sequence_index)`
/// to ensure deterministic reconstruction of agent behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalEvent {
    /// ERC-6551 token address representing the agent identity.
    pub agent_id: Address,

    /// Transaction nonce from the agent's wallet.
    /// Ensures causal ordering across transactions.
    pub nonce: u64,

    /// Sequence index within the same transaction.
    /// Handles multiple events in a single transaction.
    pub sequence_index: u32,

    /// Semantic event type (e.g., "task_executed", "payment_received").
    pub event_type: String,

    /// Optional unique task identifier for traceability.
    pub task_id: Option<String>,

    /// Execution result: "success", "failure", "partial", "timeout".
    pub result: String,

    /// Unix timestamp of event occurrence.
    pub timestamp: u64,

    /// Optional IPFS hash pointing to detailed metadata
    /// (e.g., input parameters, output data, error logs).
    pub metadata_hash: Option<H256>,
}

impl CausalEvent {
    /// Creates a new builder for constructing a `CausalEvent`.
    pub fn builder() -> CausalEventBuilder {
        CausalEventBuilder::default()
    }

    /// Validates the event fields according to causal model rules.
    ///
    /// # Validation Rules
    /// - `result` must be one of: "success", "failure", "partial", "timeout"
    /// - `agent_id` must not be the zero address
    ///
    /// # Returns
    /// `Ok(())` if valid, `Err(CausalError)` otherwise.
    pub fn validate(&self) -> Result<()> {
        // Validate result value
        if !VALID_RESULTS.contains(&self.result.as_str()) {
            return Err(CausalError::InvalidResult(self.result.clone()));
        }

        // Validate agent address is not zero
        if self.agent_id == Address::zero() {
            return Err(CausalError::InvalidAgentAddress(
                "Agent address cannot be zero".to_string(),
            ));
        }

        Ok(())
    }
}

impl PartialOrd for CausalEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CausalEvent {
    /// Compare events by causal ordering.
    ///
    /// Primary: nonce (transaction order)
    /// Secondary: sequence_index (within transaction)
    fn cmp(&self, other: &Self) -> Ordering {
        self.nonce
            .cmp(&other.nonce)
            .then_with(|| self.sequence_index.cmp(&other.sequence_index))
    }
}

/// Builder for constructing `CausalEvent` instances.
#[derive(Debug, Default)]
pub struct CausalEventBuilder {
    agent_id: Option<Address>,
    nonce: Option<u64>,
    sequence_index: Option<u32>,
    event_type: Option<String>,
    task_id: Option<String>,
    result: Option<String>,
    timestamp: Option<u64>,
    metadata_hash: Option<H256>,
}

impl CausalEventBuilder {
    /// Sets the agent identity (ERC-6551 token address).
    pub fn agent_id(mut self, agent_id: Address) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Sets the transaction nonce.
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Sets the sequence index within the transaction.
    pub fn sequence_index(mut self, index: u32) -> Self {
        self.sequence_index = Some(index);
        self
    }

    /// Sets the semantic event type.
    pub fn event_type(mut self, event_type: impl Into<String>) -> Self {
        self.event_type = Some(event_type.into());
        self
    }

    /// Sets the optional task identifier.
    pub fn task_id(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    /// Sets the execution result.
    pub fn result(mut self, result: impl Into<String>) -> Self {
        self.result = Some(result.into());
        self
    }

    /// Sets the Unix timestamp.
    pub fn timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the optional metadata hash (IPFS).
    pub fn metadata_hash(mut self, hash: H256) -> Self {
        self.metadata_hash = Some(hash);
        self
    }

    /// Builds and validates the `CausalEvent`.
    ///
    /// # Errors
    /// Returns `CausalError` if required fields are missing or validation fails.
    pub fn build(self) -> Result<CausalEvent> {
        let event = CausalEvent {
            agent_id: self.agent_id.ok_or_else(|| {
                CausalError::InvalidAgentAddress("agent_id is required".to_string())
            })?,
            nonce: self.nonce.unwrap_or(0),
            sequence_index: self.sequence_index.unwrap_or(0),
            event_type: self.event_type.unwrap_or_else(|| "unknown".to_string()),
            task_id: self.task_id,
            result: self.result.unwrap_or_else(|| "success".to_string()),
            timestamp: self.timestamp.unwrap_or(0),
            metadata_hash: self.metadata_hash,
        };

        event.validate()?;
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_address() -> Address {
        "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap()
    }

    #[test]
    fn test_event_creation_with_builder() {
        let event = CausalEvent::builder()
            .agent_id(test_address())
            .nonce(1)
            .sequence_index(0)
            .event_type("task_executed")
            .result("success")
            .timestamp(1700000000)
            .build()
            .unwrap();

        assert_eq!(event.agent_id, test_address());
        assert_eq!(event.nonce, 1);
        assert_eq!(event.sequence_index, 0);
        assert_eq!(event.event_type, "task_executed");
        assert_eq!(event.result, "success");
    }

    #[test]
    fn test_invalid_result_rejected() {
        let result = CausalEvent::builder()
            .agent_id(test_address())
            .result("invalid_result")
            .build();

        assert!(matches!(result, Err(CausalError::InvalidResult(_))));
    }

    #[test]
    fn test_zero_address_rejected() {
        let result = CausalEvent::builder()
            .agent_id(Address::zero())
            .result("success")
            .build();

        assert!(matches!(result, Err(CausalError::InvalidAgentAddress(_))));
    }

    #[test]
    fn test_causal_ordering() {
        let event1 = CausalEvent::builder()
            .agent_id(test_address())
            .nonce(1)
            .sequence_index(0)
            .build()
            .unwrap();

        let event2 = CausalEvent::builder()
            .agent_id(test_address())
            .nonce(1)
            .sequence_index(1)
            .build()
            .unwrap();

        let event3 = CausalEvent::builder()
            .agent_id(test_address())
            .nonce(2)
            .sequence_index(0)
            .build()
            .unwrap();

        assert!(event1 < event2);
        assert!(event2 < event3);
        assert!(event1 < event3);
    }
}
