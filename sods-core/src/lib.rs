//! # SODS Core
//!
//! **Symbolic On-Demand Verification over Decentralized Summaries**
//!
//! This crate implements Layer 0 of the SODS protocol: the Symbolic Core.
//! It converts Ethereum-compatible EVM logs into behavioral symbols,
//! constructs Behavioral Merkle Trees (BMTs), and generates cryptographically
//! verifiable proofs.
//!
//! ## Features
//!
//! - **Deterministic**: Same input → same BMT root across all environments
//! - **Minimal**: No network I/O, no async, focused on core crypto
//! - **Spec-compliant**: Follows [SODS RFC v0.2](https://github.com/logiccrafterdz/sods-protocol)
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use sods_core::{SymbolDictionary, BehavioralMerkleTree, BehavioralSymbol};
//!
//! // Create symbol dictionary with core symbols
//! let dict = SymbolDictionary::default();
//!
//! // Parse logs into behavioral symbols
//! let symbols = vec![
//!     BehavioralSymbol::new("Tf", 0),
//!     BehavioralSymbol::new("Dep", 1),
//! ];
//!
//! // Build Behavioral Merkle Tree
//! let bmt = BehavioralMerkleTree::new(symbols);
//! let root = bmt.root();
//!
//! // Generate and verify proofs
//! if let Some(proof) = bmt.generate_proof("Tf", 0) {
//!     assert!(proof.verify(&root));
//! }
//! ```

pub mod dictionary;
pub mod error;
pub mod proof;
pub mod symbol;
pub mod tree;
pub mod pattern;
pub mod causal_tree;

// Re-export main types for convenience
pub use dictionary::SymbolDictionary;
pub use error::SodsError;
pub use proof::Proof;
pub use symbol::BehavioralSymbol;
pub use tree::BehavioralMerkleTree;
pub use causal_tree::CausalMerkleTree;
pub mod shadow;
pub use shadow::BehavioralShadow;
pub mod plugins;
pub mod header_anchor;
pub mod ssz;
pub mod commitment;
pub use commitment::BehavioralCommitment;
pub use plugins::SymbolPlugin;
