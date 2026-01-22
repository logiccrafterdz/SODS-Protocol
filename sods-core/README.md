# sods-core

**SODS Protocol Layer 0: Symbolic Core for Behavioral Merkle Trees**

A safe, efficient, and spec-compliant Rust crate that converts Ethereum-compatible EVM logs into behavioral symbols, constructs Behavioral Merkle Trees (BMTs), and generates cryptographically verifiable proofs.

## Features

- **Deterministic**: Same input → same BMT root across all environments
- **Minimal**: No network I/O, no async, focused on core crypto
- **Spec-compliant**: Follows [SODS RFC v0.2](../spec/SODS-RFC-v0.2.md)
- **Safe**: Zero unsafe code

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sods-core = { path = "../sods-core" }
```

## Quick Start

```rust
use sods_core::{SymbolDictionary, BehavioralMerkleTree, BehavioralSymbol};

// Create symbol dictionary with core symbols
let dict = SymbolDictionary::default();

// Parse logs into behavioral symbols
let symbols = vec![
    BehavioralSymbol::new("Tf", 0),
    BehavioralSymbol::new("Dep", 1),
];

// Build Behavioral Merkle Tree
let bmt = BehavioralMerkleTree::new(symbols);
let root = bmt.root();

// Generate and verify proofs
if let Some(proof) = bmt.generate_proof("Tf", 0) {
    assert!(proof.verify(&root));
}
```

## Core Types

| Type | Description |
|------|-------------|
| `SymbolDictionary` | Maps EVM event topics to symbol codes |
| `BehavioralSymbol` | Parsed behavioral event with canonical ordering |
| `BehavioralMerkleTree` | Merkle tree over sorted symbols |
| `Proof` | Merkle inclusion proof with verification |

## Symbol Registry

| Symbol | Event | Description |
|--------|-------|-------------|
| `Tf` | Transfer | ERC20 token transfer |
| `Dep` | Deposit | WETH deposit (wrap ETH) |
| `Wdw` | Withdrawal | WETH withdrawal (unwrap ETH) |
| `Sw` | Swap | Uniswap V2 swap |
| `LP+` | Mint | Add liquidity |
| `LP-` | Burn | Remove liquidity |

## Testing

```bash
cargo test          # Run all tests
cargo clippy        # Run lints
cargo doc --open    # View documentation
```

## License

[CC0 1.0 Universal](LICENSE) — Public Domain