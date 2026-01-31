//! In-memory causal event recorder for SODS Agent Registry.
//!
//! This module provides `CausalEventRecorder`, which stores and validates
//! causal events for multiple agents while enforcing strict causal ordering.
//!
//! # Example
//!
//! ```rust
//! use sods_causal::{CausalEvent, CausalEventRecorder};
//! use ethers::types::Address;
//!
//! let mut recorder = CausalEventRecorder::new();
//!
//! let agent: Address = "0x1234567890123456789012345678901234567890".parse().unwrap();
//!
//! // Record first event (nonce=0, sequence=0)
//! let event = CausalEvent::builder()
//!     .agent_id(agent)
//!     .nonce(0)
//!     .sequence_index(0)
//!     .event_type("task_executed")
//!     .result("success")
//!     .timestamp(1700000000)
//!     .build()
//!     .unwrap();
//!
//! recorder.record_event(event).unwrap();
//!
//! assert_eq!(recorder.get_agent_events(&agent).unwrap().len(), 1);
//! ```

use std::collections::HashMap;

use ethers::types::Address;

use crate::error::{CausalError, Result};
use crate::event::CausalEvent;

/// In-memory recorder for causal events across multiple agents.
///
/// The recorder enforces causal ordering by validating that:
/// - Nonces are contiguous within an agent's event history
/// - Sequence indices are contiguous within the same nonce
///
/// # Thread Safety
/// This struct is not thread-safe by default. Wrap in `Arc<Mutex<>>` for concurrent access.
#[derive(Debug, Default)]
pub struct CausalEventRecorder {
    /// Events stored per agent (keyed by agent address)
    events: HashMap<Address, Vec<CausalEvent>>,
}

impl CausalEventRecorder {
    /// Creates a new empty recorder.
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
        }
    }

    /// Records a new causal event after validation.
    ///
    /// # Validation
    /// - Event fields must pass `CausalEvent::validate()`
    /// - Causal ordering must be maintained (no nonce or sequence gaps)
    ///
    /// # Errors
    /// Returns `CausalError` if validation fails or ordering is violated.
    pub fn record_event(&mut self, event: CausalEvent) -> Result<()> {
        // Validate event fields
        event.validate()?;

        // Ensure causal ordering
        self.ensure_causal_ordering(&event)?;

        // Store event
        self.events
            .entry(event.agent_id)
            .or_default()
            .push(event);

        Ok(())
    }

    /// Retrieves all recorded events for a specific agent.
    ///
    /// Returns `None` if no events exist for the agent.
    pub fn get_agent_events(&self, agent_id: &Address) -> Option<&Vec<CausalEvent>> {
        self.events.get(agent_id)
    }

    /// Returns the number of agents with recorded events.
    pub fn agent_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the total number of events across all agents.
    pub fn total_events(&self) -> usize {
        self.events.values().map(|v| v.len()).sum()
    }

    /// Clears all recorded events.
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Ensures causal ordering for the new event.
    ///
    /// Rules:
    /// - First event for an agent must have nonce=0, sequence_index=0
    /// - Subsequent events must either:
    ///   a) Continue the same nonce with sequence_index incremented by 1
    ///   b) Start a new nonce (incremented by 1) with sequence_index=0
    fn ensure_causal_ordering(&self, event: &CausalEvent) -> Result<()> {
        let agent_events = match self.events.get(&event.agent_id) {
            Some(events) if !events.is_empty() => events,
            _ => {
                // First event for this agent
                if event.nonce != 0 {
                    return Err(CausalError::NonceGap {
                        expected: 0,
                        actual: event.nonce,
                    });
                }
                if event.sequence_index != 0 {
                    return Err(CausalError::SequenceGap {
                        expected: 0,
                        actual: event.sequence_index,
                    });
                }
                return Ok(());
            }
        };

        let last_event = agent_events.last().unwrap();

        if event.nonce == last_event.nonce {
            // Same transaction: sequence_index must be contiguous
            let expected_seq = last_event.sequence_index + 1;
            if event.sequence_index != expected_seq {
                return Err(CausalError::SequenceGap {
                    expected: expected_seq,
                    actual: event.sequence_index,
                });
            }
        } else if event.nonce == last_event.nonce + 1 {
            // New transaction: sequence_index must start at 0
            if event.sequence_index != 0 {
                return Err(CausalError::SequenceGap {
                    expected: 0,
                    actual: event.sequence_index,
                });
            }
        } else {
            // Nonce gap detected
            return Err(CausalError::NonceGap {
                expected: last_event.nonce + 1,
                actual: event.nonce,
            });
        }

        Ok(())
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

    fn test_address_2() -> Address {
        "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
            .parse()
            .unwrap()
    }

    fn create_event(agent: Address, nonce: u64, seq: u32) -> CausalEvent {
        CausalEvent::builder()
            .agent_id(agent)
            .nonce(nonce)
            .sequence_index(seq)
            .event_type("test")
            .result("success")
            .timestamp(1700000000)
            .build()
            .unwrap()
    }

    #[test]
    fn test_record_first_event() {
        let mut recorder = CausalEventRecorder::new();
        let event = create_event(test_address(), 0, 0);

        assert!(recorder.record_event(event).is_ok());
        assert_eq!(recorder.get_agent_events(&test_address()).unwrap().len(), 1);
    }

    #[test]
    fn test_sequence_within_nonce() {
        let mut recorder = CausalEventRecorder::new();

        recorder
            .record_event(create_event(test_address(), 0, 0))
            .unwrap();
        recorder
            .record_event(create_event(test_address(), 0, 1))
            .unwrap();
        recorder
            .record_event(create_event(test_address(), 0, 2))
            .unwrap();

        assert_eq!(recorder.get_agent_events(&test_address()).unwrap().len(), 3);
    }

    #[test]
    fn test_nonce_transition() {
        let mut recorder = CausalEventRecorder::new();

        recorder
            .record_event(create_event(test_address(), 0, 0))
            .unwrap();
        recorder
            .record_event(create_event(test_address(), 1, 0))
            .unwrap();

        assert_eq!(recorder.get_agent_events(&test_address()).unwrap().len(), 2);
    }

    #[test]
    fn test_sequence_gap_rejected() {
        let mut recorder = CausalEventRecorder::new();

        recorder
            .record_event(create_event(test_address(), 0, 0))
            .unwrap();

        let result = recorder.record_event(create_event(test_address(), 0, 2));
        assert!(matches!(result, Err(CausalError::SequenceGap { .. })));
    }

    #[test]
    fn test_nonce_gap_rejected() {
        let mut recorder = CausalEventRecorder::new();

        recorder
            .record_event(create_event(test_address(), 0, 0))
            .unwrap();

        let result = recorder.record_event(create_event(test_address(), 2, 0));
        assert!(matches!(result, Err(CausalError::NonceGap { .. })));
    }

    #[test]
    fn test_multiple_agents_isolation() {
        let mut recorder = CausalEventRecorder::new();

        // Agent 1: nonce 0
        recorder
            .record_event(create_event(test_address(), 0, 0))
            .unwrap();

        // Agent 2: also nonce 0 (independent)
        recorder
            .record_event(create_event(test_address_2(), 0, 0))
            .unwrap();

        assert_eq!(recorder.agent_count(), 2);
        assert_eq!(recorder.total_events(), 2);
    }
}
