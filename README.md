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
- Exponential backoff retry (500ms, 1.5s, 4s)
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





# List supported symbols
sods symbols

# List supported chains
sods chains

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
  - `ListNFT`: Blur NFT listings (OrdersMatched)
  - `BridgeOut`: L2→L1 withdrawals (Arbitrum OutboundTransfer, Scroll MessageSent)
  - `Frontrun` / `Backrun`: MEV pattern presets for frontrun (Tf→Sw) and backrun (Sw→Tf) detection
- **Deployer Detection**: RPC integration to identify contract deployers for rug pull detection (`from == deployer` condition)

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
The verifier now outputs a **Confidence Score (0.0 - 1.0)** for every detection, rewarding:
-  Valid Merkle Proofs (Base)
-  Signed Transactions (+0.2)
-  Deployer Actions (+0.3)
-  Value Transfers (+0.1)

## Status

- Specification: **v1.0** (Stable)
- PoC: **v0.1** (Sepolia testnet)
- sods-core: **v0.2.0** (Rust crate)
- sods-verifier: **v0.2.0** (Rust crate)
- sods-p2p: **v0.2.0** (Rust crate)
- sods-cli: **v1.1.0** (Rust binary)
- Stage: **v1.1** / Production-Ready L2 Support
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
├── sods-p2p/           <- Layer 2: P2P Network (Rust)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── peer.rs
│       ├── client.rs
│       ├── behavior.rs
│       ├── protocol.rs
│       ├── consensus.rs
│       └── ...
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
│           └── daemon.rs
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
