# SODS Protocol

**Symbolic On-Demand Verification over Decentralized Summaries**

SODS is an experimental protocol proposal that explores a new way to *read* blockchains.

Instead of indexing or scraping raw on-chain data, SODS proposes verifying **behavioral claims**
(e.g. swaps, liquidity events) using symbolic commitments and Merkle proofs —
without relying on centralized indexers or archive nodes.

---

## Getting Started

**[ Read the Full Getting Started Guide](GETTING_STARTED.md)**

## Proof of Concept (PoC)

We've built a minimal PoC that verifies behavioral patterns in Sepolia blocks — with **202-byte proofs** and **$0 cost**.

### Results

| Symbol | Meaning              | Proof Size | Verification Time |
|--------|----------------------|------------|-------------------|
| `Tf`   | ERC20 Transfer       | 202 bytes  | < 1 ms            |
| `Dep`  | WETH Deposit         | 202 bytes  | < 1 ms            |
| `Wdw`  | WETH Withdrawal      | 202 bytes  | < 1 ms            |
| `Sw`   | Uniswap Swap         | 202 bytes  | < 1 ms            |
| `LP+`  | Add Liquidity        | 202 bytes  | < 1 ms            |
| `LP-`  | Remove Liquidity     | 202 bytes  | < 1 ms            |
| `MintNFT` | NFT Mint          | 202 bytes  | < 1 ms            |
| `BuyNFT`  | NFT Purchase (Seaport) | 202 bytes | < 1 ms         |
| `ListNFT` | NFT Listing (Blur) | 202 bytes  | < 1 ms            |
| `BridgeIn` | L1→L2 Deposit     | 202 bytes  | < 1 ms            |
| `BridgeOut` | L2→L1 Withdrawal | 202 bytes  | < 1 ms            |
| `AAOp`  | ERC-4337 UserOp      | 202 bytes  | < 1 ms            |
| `Permit2`| Gasless Approval    | 202 bytes  | < 1 ms            |
| `CoWTrade`| CoW Swap Intent    | 202 bytes  | < 1 ms            |

**[See the full PoC results and code](poc/)**

---

## Rust Implementation

The protocol is being implemented as a set of Rust crates:

### sods-core (Layer 0)

The symbolic core for Behavioral Merkle Trees. Handles:

- EVM log parsing to behavioral symbols
- Merkle tree construction
- Proof generation and verification

```rust
use sods_core::{SymbolDictionary, BehavioralMerkleTree, BehavioralSymbol};

let symbols = vec![
    BehavioralSymbol::new("Tf", 0, vec![]),
    BehavioralSymbol::new("Dep", 1, vec![]),
];

let bmt = BehavioralMerkleTree::new(symbols);
let proof = bmt.generate_proof("Tf", 0).unwrap();
assert!(proof.verify(&bmt.root()));
```

### sods-verifier (Layer 1)

Local verification using public RPC endpoints. Handles:

- RPC data fetching with LRU caching (100 blocks)
- Multi-provider failover (Failover across ≥3 diverse endpoints)
- Adaptive RPC (Exponential backoff for rate limit handling)
- L2-Aware Resilience (Specialized backoff profiles for L2 chains)
- Pre-flight health checks
- Symbol validation
- End-to-end verification with timing metrics

```rust
use sods_verifier::BlockVerifier;

let verifier = BlockVerifier::new("https://sepolia.infura.io/v3/YOUR_KEY")?;

let result = verifier
    .verify_symbol_in_block("Dep", 10002322)
    .await?;

println!("Verified: {}", result.is_verified);
println!("Proof size: {} bytes", result.proof_size_bytes);
```

### sods-p2p (Layer 2)

P2P proof exchange and social consensus using libp2p. Handles:

- Peer discovery via identify protocol
- Cryptographically signed proof responses (secp256k1)
- Proof exchange via request-response protocol
- Social consensus verification (2/3 majority)
- Peer reputation tracking

```rust
use sods_p2p::{SodsClient, SodsPeer};

// Client P2P verification with RPC fallback
let mut client = SodsClient::with_fallback("https://sepolia.infura.io/v3/KEY")?;

let result = client
    .verify_via_p2p("Dep", 10002322)
    .await?;

println!("Verified: {}", result.is_verified);
println!("Agreeing peers: {}", result.agreeing_peers);
```

### sods-zk (Layer 2.5)

Zero-Knowledge Behavioral Proofs using RISC Zero. Handles:

- Generating STARK proofs of pattern matches
- Privacy-preserving verification (only true/false public output)
- On-chain verification compatibility

```rust
use sods_zk::prove_behavior;

let receipt = prove_behavior(symbols, "LP+ -> Sw -> LP-")?;
let valid: bool = receipt.journal.decode()?;
```

### sods-cli (Layer 3)

Command-line interface for SODS Protocol. Provides:

- Terminal-first verification commands
- Human-readable and JSON output modes
- Multi-chain support with smart defaults
  - **Ethereum Source**: Mainnet, Sepolia
  - **L2 Support**: Arbitrum, Base, Optimism, Polygon zkEVM, Scroll

```bash
# Verify a symbol in a block
sods verify Dep --block 10002322 --chain sepolia

# Verify Behavioral Patterns (Sequences)
# Check for "Liquidity Add -> Swap -> Liquidity Remove"
sods verify "LP+ -> Sw -> LP-" --block 12345678 --chain base

# Verify Patterns with Quantifiers
# Check for "At least 2 Transfers" or "1 to 3 Approvals" (DSL: {n}, {n,}, {n,m})
sods verify "Tf{2,}" --block 12345678 --chain arbitrum
sods verify "Appr{1,3} -> Tf" --block 12345678 --chain optimism

# Verify on L2s (Arbitrum, Base, Optimism, Polygon zkEVM, Scroll)
sods verify Tf --block 170000000 --chain arbitrum
sods verify Tf --block 9000000 --chain base

# Discover behavioral hotspots (Find active blocks)
sods discover --symbol Sw --chain base --last 20

# Detect behavioral trends (New in v1.1)
sods trend --pattern "LP+ -> Sw" --chain base --window 10

# Autonomous monitoring (Continuous watchdog)
sods monitor --pattern "Sw{3,}" --chain base --interval 30s

# Run as background daemon with community threat feed (and webhooks)
sods daemon start --threat-feed "https://raw.githubusercontent.com/sods/threats/main/base.json" --chain base --webhook-url "https://ntfy.sh/my_alerts" --autostart

# Monitor Next-Gen Activity (New in v2.2)
sods verify AAOp --block 20000000 --chain ethereum
sods trend --pattern "Permit2" --chain base --window 50
sods verify "CoWTrade" --block 20000000 --chain ethereum





# List supported symbols
sods symbols

# List supported chains
sods chains

# Generate ZK Behavioral Proof (New in v2.5)
sods zk-prove --pattern "Sandwich" --block 20000000 --chain ethereum

# JSON output for scripting
sods verify Tf --block 10002322 --json
```

---

## What SODS is NOT

- Not an indexer
- Not a data analytics platform
- Not a replacement for archive nodes
- Not a finalized standard

## What's New in v1.1 

- **L2 Native Support**: Direct verification on Arbitrum, Base, Optimism, Scroll, and Polygon zkEVM.
- **Discovery Engine**: New `sods discover` command to find behavioral hotspots in recent blocks.
- **System Service**: Run as a daemon with `sods daemon` (Linux/macOS).
- **Secure Webhooks**: Forward alerts to ntfy.sh, Discord, or Telegram with privacy guarantees.
- **Threat Intelligence**: Subscribe to community blocklists via HTTP feeds (v1.1) or **P2P Gossipsub** (New in v1.2).
- **Sybil Immunity**: Adaptive consensus via **Proof-of-Behavioral-Stake (PoBS)** using decaying reputation.
- **Causal Behavioral Proofs**: Cryptographically verify that event sequences (`Tf -> Sw`) are executed by a single actor in order using **Causal Merkle Trees**.
- **Predictive Behavioral Shadowing**: Proactively monitor actor states (Shadows) to detect high-risk pattern initiations (`LP+`) and alert on deviations before completion.
- **Real-Time Mempool Monitoring**: Intercept pending transactions and detect threats before they are mined.
- **Regression Testing**: Automated CI integration tests for multi-chain support.

## What's New in v1.2 (Latest)

- **Block Header Anchoring**: Cryptographically anchors all logs to block headers via receipt trie proofs. Eliminates blind trust in RPC providers (Trustless Mode).
- **Adaptive RPC**: Self-healing connection logic that automatically throttles requests when rate limits are detected (Adaptive Backoff).
- **Dynamic Symbol Loading**: Extensible plugin system to load new behavioral symbols from JSON definitions (URL/File) without recompiling.
- **Enhanced Monitoring**: `sods monitor` now supports auto-adaptation and custom plugin loading at runtime.
- **New Behavioral Symbols**:
  - `MintNFT`: ERC721/ERC1155 Mint (Transfer from 0x0)
  - `BuyNFT`: Seaport NFT purchases (OrderFulfilled)
  - `ListNFT`: Blur NFT listings (OrdersMatched)
  - `BridgeIn`: L1→L2 bridge deposits (Optimism DepositFinalized, Scroll FinalizeDepositERC20)
  - `BridgeOut`: L2→L1 withdrawals (Arbitrum OutboundTransfer, Scroll MessageSent/WithdrawalInitiated)
  - `Frontrun` / `Backrun`: MEV pattern presets for frontrun (Tf→Sw) and backrun (Sw→Tf) detection
- **Production L2 Validation**: Confirmed behavioral symbol extraction on Scroll Mainnet and Polygon zkEVM Mainnet using on-chain blocks.
- **Deployer Detection**: Advanced infrastructure to identify contract deployers for rug pull detection (`from == deployer` condition) with LRU caching.

- **Connection Health Checks**: Pre-flight validation of RPC health before starting long-running monitoring or trend sessions.

## What's New in v1.4 (Latest)

- **On-Chain Behavioral Proofs**: SODS can now generate proofs that are verifiable inside Ethereum smart contracts.
- **Solidity Verifier Library**: `SODSVerifier.sol` enables DeFi protocols to natively react to verified on-chain behaviors (Rug Pulls, MEV, etc.).
- **ABI-Encoded Export**: New `sods export-proof` command generates hex-encoded calldata for direct contract interaction.
- **EVM-Friendly Hashing**: Introduced Keccak256 tree construction and leaf hashing to match Solidity's native hashing rules.
- **Trustless Behavioral Oracle**: Transforms SODS from an off-chain analysis tool into a reactive, trustless behavioral guard.

## What's New in v1.5 (Deep Verification)

- **Cryptographically Correct Receipt Trie**: Replaced placeholder trie computation with an accurate implementation of the Ethereum Ordered Patricia Trie.
- **EIP-4844 Support**: Deep verification now supports Type 3 (Blob) transactions used by modern L2 solutions.
- **Trustless Mode Verification**: SODS now locally recomputes the receipt trie root to verify log authenticity against the block header's `receiptsRoot`.
- **CLI Support**: Enforce deep cryptographic verification using the `--mode trustless` flag.
- **Improved Security**: Native detection of RPC tampering; SODS will fail verification if the provided logs do not match the on-chain consensus.

## What's New in v2.1 (Hardening)

- **P2P Sybil Resistance (Proof-of-Behavior)**: New peers must solve a behavioral puzzle (verifying a random block) to gain a "Reliable" status. P2P trust is now earned, not granted.
- **Hardened ABI Encoding (v2.0)**: Replaced manual byte manipulation with the `ethabi` crate for 100% Solidity compatibility. Ensures all exported proofs are perfectly decoded by `SODSVerifier.sol`.
- **Dynamic L2 Event Resolution**: Moving from hardcoded topic hashes to dynamic signature hashing. SODS is now resilient to bridge contract redeploys and upgrades on L2s (Scroll, Polygon zkEVM, etc.).
- **Daemon Memory Leak Fix (GC)**: Periodic garbage collection of expired monitoring rules (every 5 minutes). Long-running daemons now maintain a stable memory footprint.
- **Customizable Expiration**: New `--expire-after` flag for the daemon to automatically prune old threat reports.

## What's New in v2.2 (Next-Gen Dictionary)

- **ERC-4337 Support (`AAOp`)**: Detect and verify Account Abstraction UserOperation executions with `user_op_hash` context.
- **Permit2 Support (`Permit2`)**: Monitor gasless token approvals and extract expiration deadlines.
- **Intent-Based Fulfillments (`CoWTrade`)**: Verify CoW Swap trade fulfillments directly from settlement events.
- **Enriched Behavioral Metadata**: `BehavioralSymbol` now natively supports `user_op_hash`, `permit_deadline`, and `solver` fields.
- **Expanded L2 Dictionary**: Canonical support for next-gen events on Base, Arbitrum, Optimism, and Scroll.

## What's New in v2.3 (Security Hardening)

- **EIP-712 Structured Signing**: Hardened on-chain verification in `SODSVerifier.sol` using domain-separated signatures. Prevents cross-protocol replay attacks.
- **Randomized P2P Challenges**: Peer-of-Behavior puzzles are now randomized per-peer to eliminate pre-computation exploits and ensure genuine Sybil resistance.
- **Thread-Safe Daemonization**: Fixed CLI startup sequence to perform `daemonize` before spawning async threads, resolving long-standing stability issues on Linux/macOS.
- **Webhook Pattern Privacy**: Pattern hashes in alerts are now salted with a per-boot random secret, preventing brute-force reverse-engineering of monitored behaviors.
- **Automated Rule Maintenance**: Implemented storage pruning for the P2P threat registry to prevent unbounded disk growth.

## What's New in v3.0 (Zero-RPC Verification)

- **Zero-RPC Verification**: First truly trustless behavioral verification system that eliminates reliance on `eth_getLogs`. 
- **Ethereum Storage Proofs**: Uses standard `eth_getProof` and Merkle-Patricia Trie (MPT) validation to prove receipt data directly from Ethereum's state trie.
- **Local MPT Verifier**: Re-implemented Ethereum's MPT verification logic in `sods-core` to validate path proofs against block header `receiptsRoot`.
- **Granular Trustless Mode**: Use `--mode storage-proof` for single-receipt cryptographic proof or `--mode trustless` for bulk header anchoring.
- **Improved L2 Support**: Enhanced receipt RLP parsing for Arbitrum and Optimism to support deep verification across major rollups.

## Hybrid Trust Model Enhancement

- **Local Truth Supremacy**: Absolute priority for local verification results. If a symbol is verified locally, P2P consensus is ignored, preventing eclipse or collusion attacks.
- **Adaptive Quorum**: Dynamic consensus thresholds that scale with network size (100% for bootstrap, 67% for medium, 60% for large networks).
- **Immediate Slashing**: Malicious peers providing proofs that contradict verified local truth are automatically blacklisted.
- **WebRTC Transport**: Support for browser-compatible and mobile-friendly P2P connections via WebRTC.

## High-Performance Verification Engine

- **Source-Level Symbol Filtering**: Drastically reduces bandwidth by fetching ONLY the logs relevant to the requested pattern using Ethereum topic filters.
- **Incremental BMT Engine**: Optimized Merkle tree construction for sparse symbol sets, reducing memory footprint to < 10MB.
- **Pattern Caching**: Sub-millisecond response times for repeated behavioral queries via a built-in LRU cache.
- **Real-Time Speed**: Verification of complex patterns in 10K+ log blocks now completes in **< 200ms**.

## ZK Behavioral Proofs (Early Access)

- **Privacy-Preserving Verification**: Prove behaviors occurred without revealing sensitive metadata (addresses, amounts).
- **RISC Zero Integration**: Native support for generating STARK receipts via zkVM.
- **On-Chain ZK Verification**: Complete guide and snippets for Ethereum smart contract integration.
- **`sods zk-prove`**: New top-level CLI command for zero-knowledge proof generation.

## Behavioral Dictionary 2.0 (New!)

The protocol now supports context-aware behavioral analysis with **Metadata**, **MEV Patterns**, and **Confidence Scoring**.

### 1. Context-Aware Symbols
Symbols now carry rich metadata to enable deeper analysis:
- `Tf` (Transfer): `from`, `to`, `value`
- `MintNFT` / `BuyNFT` / `ListNFT`: NFT Market activity (Seaport, Blur)
- `BridgeIn` / `BridgeOut`: Cross-chain bridge deposits and withdrawals (Optimism, Arbitrum, Scroll)

### 2. MEV Pattern DSL
Detect complex MEV strategies using the new pattern language:

```bash
# Detect Sandwich Attacks (Heuristic: Transfer -> Swap -> Transfer)
sods verify "Sandwich" --block 123456

# Detect Frontrun/Backrun patterns
sods verify "Frontrun" --block 123456   # Tf -> Sw
sods verify "Backrun" --block 123456    # Sw -> Tf

# Detect Deployer Rug Pulls (Context condition)
sods verify "Tf where from == deployer" --block 123456
```

### 3. Confidence Scoring Engine
The verifier now outputs a **Confidence Score (0.0 - 1.0)** for every detection per Behavioral Dictionary 2.0 spec:
-  **Base Score**: 0.5 (Verified Merkle Proof)
-  **Signed Action**: +0.2 (Known transaction sender)
-  **Deployer Context**: +0.3 (Action initiated by contract deployer)
-  **Value Density**: +0.1 (Detection involves value transfer)
-  **Data Integrity**: -0.4 (Penalty if internal/causal transaction data is missing)

## Status

- Specification: **v0.2** (Symbolic Primitives)
- PoC: **v0.5** (Verified Proofs)
- sods-core: **v0.1.2**
- sods-verifier: **v0.1.2**
- sods-p2p: **v0.1.0**
- sods-cli: **v0.1.5**
- Stage: **Pre-Alpha / Research Initiative**
- Seeking: Technical feedback, threat analysis, edge cases

## Architecture

[SODS Architecture — Trust Model and Data Flow](ARCHITECTURE.md)

## Specification

[SODS Protocol — Specification v1.0](spec/SODS-SPEC-v1.0.md)

## Repository Structure

```
sods-protocol/
├── README.md           <- You are here
├── GETTING_STARTED.md  <- Usage guide
├── ARCHITECTURE.md     <- Trust model and data flow
├── LICENSE             <- CC0 1.0
├── spec/
│   └── SODS-RFC-v0.2.md
├── sods-core/          <- Layer 0: Symbolic Core (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── symbol.rs
│       ├── dictionary.rs
│       ├── tree.rs
│       ├── proof.rs
│       ├── pattern.rs
│       ├── header_anchor.rs
│       └── error.rs
├── sods-verifier/      <- Layer 1: Local Verifier (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── header_anchor.rs
│       ├── error.rs
│       ├── query.rs
│       ├── result.rs
│       ├── rpc.rs
│       └── verifier.rs
├── sods-zk/            <- Layer 2.5: ZK behavioral proofs (Rust)
│   ├── Cargo.toml
│   ├── src/
│   │   └── lib.rs
│   └── methods/        <- Guest programs for zkVM
│       ├── build.rs
│       ├── src/
│       └── guest/
├── sods-cli/           <- Layer 3: CLI Interface (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── output.rs
│       └── commands/
│           ├── verify.rs
│           ├── chains.rs
│           ├── symbols.rs
│           ├── discover.rs
│           ├── trend.rs
│           ├── monitor.rs
│           ├── daemon.rs
│           └── export_proof.rs
├── contracts/          <- Smart Contracts (Solidity)
│   └── SODSVerifier.sol
└── poc/                <- Python PoC
    ├── README.md
    ├── bmt_builder.py
    ├── verifier.py
    └── ...
```

## Disclaimer

This is a research proposal.
No security guarantees are claimed.
Do not use in production systems.

---

## License

[CC0 1.0 Universal](LICENSE) — Public Domain
