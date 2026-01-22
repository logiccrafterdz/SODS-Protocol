//! # SODS Verifier
//!
//! **Layer 1: Local Verification for Behavioral Merkle Trees**
//!
//! This crate enables end users to verify behavioral patterns in on-chain blocks
//! using only public RPC endpoints (Infura, Alchemy) and the `sods-core` cryptographic engine.
//!
//! ## Features
//!
//! - **User-friendly**: Simple API for common verification queries
//! - **Resilient**: Handles RPC errors and rate limits gracefully
//! - **Efficient**: Minimizes RPC calls and memory usage
//! - **Spec-compliant**: Uses `sods-core` exactly as defined in RFC v0.2
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use sods_verifier::BlockVerifier;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let verifier = BlockVerifier::new("https://sepolia.infura.io/v3/YOUR_KEY")?;
//!     
//!     let result = verifier
//!         .verify_symbol_in_block("Dep", 10002322)
//!         .await?;
//!
//!     println!("Verified: {}", result.is_verified);
//!     println!("Proof size: {} bytes", result.proof_size_bytes);
//!     println!("Total time: {:?}", result.total_time);
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod header_anchor;
pub mod query;
pub mod result;
pub mod rpc;
pub mod verifier;
pub mod mempool;

// Re-export main types for convenience
pub use error::SodsVerifierError;
pub use header_anchor::{VerificationMode, BlockHeader, AnchorValidation};
pub use query::QueryParser;
pub use result::VerificationResult;
pub use rpc::RpcClient;
pub use verifier::BlockVerifier;
pub use mempool::{MempoolMonitor, PendingAlert};
