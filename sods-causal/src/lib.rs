//! # SODS Causal
//!
//! **Causal Event Model for SODS Agent Registry**
//!
//! This crate implements a robust, minimal, and extensible causal event model
//! that captures verifiable behavioral interactions of AI agents. It provides
//! the foundational data layer for the Causal Agent Registry, enabling trustless
//! verification of agent behavior without centralized reputation systems.
//!
//! ## Features
//!
//! - **Causal Ordering**: Events are strictly ordered by `(nonce, sequence_index)`
//! - **Validation**: Comprehensive field validation for all events
//! - **Multi-Agent**: In-memory recorder supports multiple agent histories
//! - **Minimal**: Zero dependencies beyond `ethers` and `serde`
//!
//! ## Quick Start
//!
//! ```rust
//! use sods_causal::{CausalEvent, CausalEventRecorder};
//! use ethers::types::Address;
//!
//! // Create a recorder
//! let mut recorder = CausalEventRecorder::new();
//!
//! // Parse agent address
//! let agent: Address = "0x1234567890123456789012345678901234567890".parse().unwrap();
//!
//! // Record a task execution event
//! let event = CausalEvent::builder()
//!     .agent_id(agent)
//!     .nonce(0)
//!     .sequence_index(0)
//!     .event_type("task_executed")
//!     .task_id("task-001")
//!     .result("success")
//!     .timestamp(1700000000)
//!     .build()
//!     .unwrap();
//!
//! recorder.record_event(event).unwrap();
//!
//! // Retrieve agent's event history
//! let history = recorder.get_agent_events(&agent).unwrap();
//! assert_eq!(history.len(), 1);
//! ```
//!
//! ## Causal Ordering
//!
//! Events are ordered by:
//! 1. **Nonce** (primary): Transaction order on-chain
//! 2. **Sequence Index** (secondary): Order within a transaction
//!
//! This ensures deterministic reconstruction of agent behavior history.

pub mod error;
pub mod event;
pub mod recorder;
pub mod tree;
pub mod proof;

// Re-export main types for convenience
pub use error::{CausalError, Result};
pub use event::{CausalEvent, CausalEventBuilder, VALID_RESULTS};
pub use recorder::CausalEventRecorder;
pub use tree::CausalMerkleTree;
pub use proof::CausalProof;

