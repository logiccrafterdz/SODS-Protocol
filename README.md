# SODS Protocol

**Symbolic On-Demand Verification over Decentralized Summaries**

SODS is an experimental protocol proposal that explores a new way to *read* blockchains.

Instead of indexing or scraping raw on-chain data, SODS proposes verifying **behavioral claims**
(e.g. swaps, liquidity events) using symbolic commitments and Merkle proofs —
without relying on centralized indexers or archive nodes.

---

## Proof of Concept (PoC)

We've built a minimal PoC that verifies behavioral patterns in Sepolia blocks — with **202-byte proofs** and **$0 cost**.

### Results

| Symbol | Meaning              | Proof Size | Verification Time |
|--------|----------------------|------------|-------------------|
| `Tf`   | ERC20 Transfer       | 202 bytes  | < 1 ms            |
| `Dep`  | WETH Deposit         | 202 bytes  | < 1 ms            |
| `Wdw`  | WETH Withdrawal      | 202 bytes  | < 1 ms            |

**[See the full PoC results and code](poc/)**

---

## What SODS is NOT

- Not an indexer
- Not a data analytics platform
- Not a replacement for archive nodes
- Not a finalized standard

## Status

- Specification: **Draft v0.2**
- PoC: **v0.1** (Sepolia testnet)
- Stage: Experimental / Research
- Seeking: Technical feedback, threat analysis, edge cases

## Specification

[SODS Protocol — RFC v0.2](spec/SODS-RFC-v0.2.md)

## Repository Structure

```
sods-protocol/
├── README.md           <- You are here
├── LICENSE             <- CC0 1.0
├── spec/
│   └── SODS-RFC-v0.2.md
└── poc/
    ├── README.md       <- PoC results & usage
    ├── bmt_builder.py  <- BMT construction
    ├── verifier.py     <- Proof verification CLI
    ├── merkle.py       <- Merkle tree implementation
    ├── config.py       <- Configuration
    ├── proofs/         <- Generated proofs
    └── screenshots/    <- Visual results
```

## Disclaimer

This is a research proposal.
No security guarantees are claimed.
Do not use in production systems.

---

## License

[CC0 1.0 Universal](LICENSE) — Public Domain
