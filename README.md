# SODS Protocol: Trustless Behavioral Verification

[![License: CC0-1.0](https://licensebuttons.net/l/zero/1.0/80x15.png)](http://creativecommons.org/publicdomain/zero/1.0/)
[![npm version](https://img.shields.io/npm/v/sods-cli.svg)](https://www.npmjs.com/package/sods-cli)
[![Docker](https://img.shields.io/badge/docker-stable-blue.svg)](https://www.docker.com/)

SODS is NOT a blockchain indexer. It is a trustless behavioral verifier that answers one question: "Did this specific behavioral pattern occur in a given block?"

Unlike indexers that store and query all data, SODS generates cryptographic proofs for predefined behavioral patterns without centralized infrastructure or archival nodes.

## Educational Demo
-------------------

[![Watch SODS Demo](https://img.youtube.com/vi/dhE_uNHLjec/hqdefault.jpg)](https://www.youtube.com/watch?v=dhE_uNHLjec)

> Click the image above to watch a walkthrough of SODS detecting a MEV sandwich attack.

## When to Use SODS vs Alternatives

| Use Case | SODS | The Graph | Tenderly |
|----------|------|-----------|----------|
| Detect sandwich MEV attacks | Yes | No (overkill) | Yes (paid) |
| Monitor for rug pulls continuously | Yes | No (expensive) | Yes (paid) |
| Prove behavior on-chain with 202-byte proof | Yes | No | No |
| Query historical NFT trades | No | Yes | No |

SODS excels at verifying specific behavioral claims. Use indexers for general-purpose data queries.

## Quick Start Examples

```bash
# Verify if a sandwich attack occurred
sods verify "Sandwich" --block 20000000 --chain ethereum

# Monitor for large transfers continuously
sods daemon start --pattern "Tf where value > 1000 ether" --chain base

# Generate on-chain verifiable proof
sods export-proof --pattern "LP+" --block 20000000 --format calldata
```

## Core Principles

- **Zero Cost**: Operates using public RPC endpoints; no archive node required
- **Zero-RPC Mode**: Cryptographically verify logs via EIP-1186 storage proofs—no trust in RPC log data
- **Trustless Verification**: Uses cryptographic proofs anchored to block headers (Receipts Root validation)
- **Privacy-Preserving**: Zero-knowledge proofs reveal only behavioral validity
- **P2P Resilient**: Hybrid trust model prevents single points of failure

## Installation Methods

```bash
# Using npx (Recommended for JS/Node developers)
npx sods-cli verify "Sandwich" --block 20000000 --chain ethereum

# Using Docker
docker run --rm ghcr.io/logiccrafterdz/sods:latest verify "Sandwich" --block 20000000 --chain ethereum

# Mount configuration directory for daemon mode
docker run --rm -v $(pwd)/.sods:/root/.sods ghcr.io/logiccrafterdz/sods:latest daemon start --pattern "Tf"

# From source (Rust)
cargo install sods-cli

# Using npm (wrapper)
npx sods-cli verify "Sandwich" --block 20000000 --chain ethereum
```

## Platform Support

| Platform | CLI | Web Dashboard | Docker | npm Wrapper |
|----------|-----|---------------|--------|-------------|
| Linux    | ✅  | ✅            | ✅     | ✅          |
| macOS    | ✅  | ✅            | ✅     | ✅          |
| Windows  | ✅  | ✅            | ✅     | ✅          |

> **Note on Windows**: The CLI daemon mode is fully natively supported on Windows following the deprecation of Unix-only `daemonize` modules in favor of pure cross-platform background processes.

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

## Web Dashboard: SODS-X

The SODS Protocol includes a Web Dashboard built with React and Vite. This dashboard provides a visual interface for submitting behavioral verification queries and viewing results.

### Features
- **Local Proxy Architecture**: Uses a local Node.js `server.js` proxy to securely relay commands to the native Rust `sods-cli` without exposing your system.
- **Real-Time Verification**: Submit Merkle verification commands directly from the dashboard and view structured logs.

### Running the Web Dashboard
```bash
# 1. Start the local CLI API daemon 
cargo run -p sods-cli --bin sods -- daemon start

# 2. Start the Node.js Proxy Server
cd sods-web
node server.js

# 3. Start the Vite React Frontend
npm run dev
```

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
    BehavioralSymbol::new("Tf", 0),
    BehavioralSymbol::new("Dep", 1),
];

let bmt = BehavioralMerkleTree::new(symbols);
let proof = bmt.generate_proof("Tf", 0).unwrap();
assert!(proof.verify(&bmt.root()));
```

### sods-verifier (Layer 1)

Local verification using public RPC endpoints. Handles:

- RPC data fetching with LRU caching (100 blocks)
- **Zero-RPC Mode**: Optional verification via EIP-1186 storage proofs (No-Log mode)
- Multi-provider failover (Failover across ≥3 diverse endpoints)
- Adaptive RPC (Exponential backoff for rate limit handling)
- L2-Aware Resilience (Verified RLP encoding for Arbitrum/Optimism receipts roots)
- Pre-flight health checks
- Symbol validation
- End-to-end verification with timing metrics

```rust
use sods_verifier::BlockVerifier;

let urls = vec!["https://ethereum-sepolia.publicnode.com".to_string()];
let verifier = BlockVerifier::new(&urls)?;

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
let urls = vec!["https://ethereum-sepolia.publicnode.com".to_string()];
let mut client = SodsClient::with_fallback(&urls)?;

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

> [!NOTE]
> ZK features are now **optional** (`zk` feature flag) to ensure cross-platform buildability on Windows and other non-Unix systems.

### sods-causal (Layer 0.5)

The causal event model for agent behavior. Handles:

- Atomic behavioral event definitions
- Strict causal ordering via nonces and sequence indices
- Multi-agent event history recording and validation

```rust
use sods_causal::{CausalEvent, CausalEventRecorder};

let mut recorder = CausalEventRecorder::new();
let event = CausalEvent::builder()
    .agent_id(agent_address)
    .nonce(0)
    .sequence_index(0)
    .event_type("task_executed")
    .result("success")
    .build()?;

recorder.record_event(event)?;

let tree = recorder.build_merkle_tree(agent_address)?;
let proof = tree.generate_proof(0);
assert!(proof.verify());
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

# Start WebSocket Alerting Server (New!)
sods daemon start --websocket-port 8080 --chain optimism

# Start Prometheus Metrics Server (New!)
sods daemon start --metrics-port 9090 --chain base

# Listen for Live Alerts (New!)
sods listen --websocket ws://localhost:8080 --pattern "Sw{3,}"

# JSON output for scripting
sods verify Tf --block 10002322 --json
```

---

## What SODS is NOT

- Not an indexer
- Not a data analytics platform
- Not a replacement for archive nodes
- Not a finalized standard

## Development History

> **Note**: For detailed, version-by-version change logs, see **[CHANGELOG.md](CHANGELOG.md)**.

The following is a summary of the key capabilities developed during the Alpha phase:

- **Multi-Chain Support**: Ethereum, Sepolia, Arbitrum, Base, Optimism, Scroll, and Polygon zkEVM
- **Trustless Header Anchoring**: Cryptographic log verification against block headers via receipt trie proofs
- **Zero-RPC Mode**: Storage proof verification via EIP-1186 (no trust in `eth_getLogs`)
- **Pattern DSL Engine**: Complex sequence matching with quantifiers (`{n,m}`) and context conditions (`where from == deployer`)
- **P2P Proof Exchange**: libp2p-based decentralized consensus with Proof-of-Behavior Sybil resistance
- **ZK Behavioral Proofs**: RISC Zero STARK integration for privacy-preserving verification
- **On-Chain Contracts**: `SODSVerifier.sol` with EIP-712 and EIP-4788 beacon root support
- **Daemon Mode**: Continuous monitoring with webhooks, Prometheus metrics, and WebSocket feeds
- **Dynamic Symbol Plugins**: Extend the behavioral dictionary at runtime via JSON definitions
- **ERC-4337, Permit2, CoW Swap**: Next-generation DeFi event support
- **Causal Behavioral Proofs (Roadmap)**: Actor-attributed event sequencing via Causal Merkle Trees. Planned for v8.0.

## Project Status

**Version: 0.2.0-beta (Pre-Production)**

> ⚠️ This project is in beta. It has been tested on Sepolia testnet but has not undergone a formal security audit.
> See [SECURITY.md](SECURITY.md) for vulnerability reporting.

License: MIT OR Apache-2.0
Documentation: See /docs directory and inline code comments

## Running Tests

To run the full E2E CLI test suite (recommended for Windows stability):

```bash
cargo test -p sods-cli --no-default-features
```

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
├── sods-causal/       <- Layer 0.5: Causal Event Model (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── event.rs
│       ├── recorder.rs
│       └── error.rs
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
│           ├── export_proof.rs
│           └── registry.rs
├── sods-web/           <- Graphical Interface: SODS-X Neural Overlay (React/Vite)
│   ├── package.json
│   ├── server.js       <- Local API Proxy to sods-cli
│   └── src/
│       ├── App.jsx     <- Extreme HUD Crystalline Components
│       └── App.css     <- Bismuth Iridescent Styling
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

This project is released into the public domain under the [CC0 1.0 Universal License](LICENSE).
