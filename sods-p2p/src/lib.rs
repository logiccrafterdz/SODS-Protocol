//! # SODS P2P
//!
//! **Layer 2: P2P Proof Exchange Network**
//!
//! This crate enables decentralized verification of behavioral patterns
//! through peer-to-peer proof exchange and social consensus.
//!
//! ## Features
//!
//! - **Peer Discovery**: mDNS-based local network discovery
//! - **Proof Exchange**: Request and serve behavioral proofs
//! - **Social Consensus**: Cross-check proofs from multiple peers
//! - **Reputation Tracking**: Track peer reliability
//! - **Fallback**: Falls back to RPC if P2P fails
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use sods_p2p::{SodsClient, SodsPeer};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Start a peer (runs in background)
//!     let mut peer = SodsPeer::new("https://sepolia.infura.io/v3/YOUR_KEY")?;
//!     tokio::spawn(async move { peer.listen("/ip4/0.0.0.0/tcp/0").await });
//!
//!     // Client requests proof via P2P
//!     let mut client = SodsClient::new()?;
//!     let result = client
//!         .verify_via_p2p("Dep", 10002322)
//!         .await?;
//!
//!     println!("Verified via P2P: {}", result.is_verified);
//!     println!("Consensus peers: {}", result.agreeing_peers);
//!
//!     Ok(())
//! }
//! ```

pub mod behavior;
pub mod cache;
pub mod client;
pub mod consensus;
pub mod error;
pub mod peer;
pub mod protocol;
pub mod reputation;
pub mod threats;
pub mod bootstrappers;
pub mod network;

// Re-export main types
pub use client::{P2pVerificationResult, SodsClient};
pub use error::SodsP2pError;
pub use peer::SodsPeer;
pub use protocol::{ProofRequest, ProofResponse};
pub use reputation::ReputationTracker;
pub use threats::{ThreatRule, ThreatRegistry};
