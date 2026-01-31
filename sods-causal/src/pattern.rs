//! Behavioral pattern matching for causal agents.
//!
//! This module defines `AgentBehaviorPattern` which allows querying agent
//! history for specific behaviors (e.g., "success{5}") and producing
//! verifiable proofs for those claims.

use std::time::Duration;
use serde::{Deserialize, Serialize};

use crate::event::CausalEvent;
use crate::tree::CausalMerkleTree;
use crate::proof::CausalBehavioralProof;
use crate::error::{CausalError, Result};

/// Defines a behavioral pattern to match against an agent's history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBehaviorPattern {
    /// Semantic event type (e.g., "task_executed").
    pub event_type: String,
    
    /// Result filter (e.g., "success", "failure").
    pub result_filter: String,
    
    /// Minimum required occurrences.
    pub min_count: u32,
    
    /// Optional maximum occurrences.
    pub max_count: Option<u32>,
    
    /// Optional time window for consideration.
    pub time_window: Option<Duration>,
}

impl AgentBehaviorPattern {
    /// Matches the pattern against a list of events.
    ///
    /// Returns the subset of events that matched the pattern.
    pub fn matches(&self, events: &[CausalEvent], now: u64) -> Vec<CausalEvent> {
        let mut filtered: Vec<CausalEvent> = events.iter()
            .filter(|e| e.event_type == self.event_type && e.result == self.result_filter)
            .filter(|e| {
                if let Some(window) = self.time_window {
                    e.timestamp >= now.saturating_sub(window.as_secs())
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        // Check constraints
        if filtered.len() < self.min_count as usize {
            return Vec::new(); // Pattern not satisfied
        }

        if let Some(max) = self.max_count {
            if filtered.len() > max as usize {
                filtered.truncate(max as usize);
            }
        }

        filtered
    }
}

/// Generates a compact behavioral proof for a given pattern.
///
/// # Arguments
/// * `tree` - The full causal Merkle tree for the agent.
/// * `pattern` - The behavioral pattern to prove.
/// * `now` - Current Unix timestamp for time-window filtering.
pub fn generate_behavioral_proof(
    tree: &CausalMerkleTree,
    pattern: &AgentBehaviorPattern,
    now: u64,
) -> Result<CausalBehavioralProof> {
    let events = tree.events();
    let matched_events = pattern.matches(events, now);

    if matched_events.is_empty() && pattern.min_count > 0 {
        return Err(CausalError::SequenceGap {
            expected: pattern.min_count,
            actual: 0,
        });
    }

    let mut event_proofs = Vec::with_capacity(matched_events.len());

    for matched in &matched_events {
        // Find index in original tree
        let index = events.iter().position(|e| e == matched)
            .ok_or_else(|| CausalError::InternalError("Event sync error".to_string()))?;
        
        event_proofs.push(tree.generate_proof(index));
    }

    Ok(CausalBehavioralProof {
        pattern: pattern.clone(),
        matched_events,
        event_proofs,
        agent_root: tree.root,
    })
}
