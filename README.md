# SODS Protocol

Symbolic On-Demand Verification over Decentralized Summaries

SODS is an experimental protocol proposal that explores a new way to *read* blockchains.

Instead of indexing or scraping raw on-chain data, SODS proposes verifying **behavioral claims**
(e.g. swaps, liquidity events) using symbolic commitments and Merkle proofs —
without relying on centralized indexers or archive nodes.

This repository contains the current RFC draft and is shared for public discussion and review.

## What SODS is NOT

- Not an indexer
- Not a data analytics platform
- Not a replacement for archive nodes
- Not a finalized standard

## Status

- Specification: Draft v0.2
- Stage: Experimental / Research
- Seeking: Technical feedback, threat analysis, edge cases

## Specification

- [SODS Protocol — RFC v0.2](spec/SODS-RFC-v0.2.md)

## Disclaimer

This is a research proposal.
No security guarantees are claimed.
Do not use in production systems.
