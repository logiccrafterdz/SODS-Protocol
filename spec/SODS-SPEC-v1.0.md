# SODS Protocol Specification v1.0

**Version:** 1.0.0  
**Status:** DRAFT (Implementation Stable)  
**Date:** 2026-01-22  
**Authors:** LogicCrafter (SODS Team)  

---

## 1. Abstract

The **Symbolic On-Demand Verification over Decentralized Summaries (SODS)** protocol enables zero-cost, zero-trust verification of on-chain behavioral patterns through cryptographic commitments to behavioral events. By abstracting raw blockchain data into "Behavioral Symbols" and organizing them into "Behavioral Merkle Trees" (BMTs), SODS allows light clients to verify event presence and sequences without downloading block bodies or relying on centralized indexers.

## 2. Motivation

### 2.1. The Problem
Current methods for verifying on-chain activity rely on imperfect tradeoffs:
- **Centralized Indexers**: Fast but demand total trust (e.g., The Graph, Etherscan API).
- **Light Clients**: Trust-minimized but resource-intensive for historical queries.
- **Archive Nodes**: prohibitively expensive and slow for pattern analysis.

### 2.2. The Solution
SODS introduces a specialized verification layer that:
1.  **Symbolizes** raw EVM logs into lightweight behavioral primitives.
2.  **Commits** to these behaviors using a Behavioral Merkle Tree (BMT).
3.  **Verifies** pattern adherence using efficient Merkle proofs (~200 bytes).

This enables "Proof of Behavior" — cryptographic evidence that specific events occurred in a block, verified purely via math and consensus. Note: Causal ordering (actor-based) is a planned enhancement.

## 3. Core Concepts

### 3.1. Behavioral Symbol
A standard unit of on-chain activity. Unlike a raw log, a symbol implies *intent*.

**Structure:**
- **Code**: `ALPHA` string (e.g., "Tf", "Sw").
- **Log Index**: Position within the block.
- **Context**: `from`, `to`, `value`, `nonce`, `tx_hash`.
- **Metadata**: `is_deployer`, `target_address`.

### 3.2. Causal Behavioral Proof (Planned)
A future proof type that validates not just the existence of an event, but its *causal ordering* relative to other events by the same actor. It proves that *Event A* happened before *Event B* in the same transaction or block scope. **Current implementation focuses on BMT-based presence and linear sequence verification.**

### 3.3. Behavioral Merkle Tree (BMT) - CURRENT
A Merkle Tree where leaves are **Behavioral Symbols**, sorted canonically by **Log Index**. This ensures that the tree structure itself reflects block-level temporal ordering.

### 3.4. Behavioral Shadow
A predictive state machine that monitors incomplete patterns. If an actor initiates a known pattern (e.g., `SandwichStart`), a "Shadow" is spawned to watch for the completion (`SandwichEnd`) or deviation.

## 4. Protocol Layers

| Layer | Component | Description | Trust Model |
|-------|-----------|-------------|-------------|
| **0** | **Symbolic Core** | Data structures, hashing rules, BMT construction. | Deterministic / Math |
| **1** | **Local Verifier** | RPC connector, log parsing, adaptive throttling. | Trust-but-Verify (RPC) |
| **2** | **P2P Network** | Threat sharing (`gossipsub`), PoBS consensus. | Honest Minority |
| **3** | **CLI / User** | Pattern DSL, Shadow monitoring, Alerts. | User Control |

### 4.1. Layer 0: Symbolic Core
The pure logic layer. It takes symbols as input and creates proofs as output. It is completely agnostic to the data source (RPC, local file, etc.).

### 4.2. Layer 1: Local Verifier
The bridge to the blockchain. It implements **Adaptive RPC Throttling**:
- **Backoff**: Doubles delay on `429` errors.
- **Decay**: Linearly reduces delay on success.
- **Jitter**: $\pm 10\%$ randomization to prevent thundering herds.

### 4.3. Layer 2: P2P Network
A decentralized layer for sharing threat intelligence.
- **Protocol**: Libp2p Gossipsub v1.1.
- **Topic**: `sods/threats/1.0.0`.
- **Sybil Resistance**: **Proof-of-Behavioral-Stake (PoBS)**. Peers gain reputation by providing valid proofs of on-chain behavior (verifying they are active chain participants).

### 4.4. Layer 3: CLI
The user interface. Supports:
- **DSL**: `verify "Sw{3,} -> Lp+"`
- **Daemon**: Background monitoring service.
- **Plugins**: Dynamic symbol loading via JSON.

## 5. Formal Specifications

### 5.1. BMT Hashing Rule
The BMT uses **SHA-256**. Leaf construction includes causal context.

```
LeafHash = SHA256(
    Symbol_Code (UTF-8) ||
    BigEndian_u64(Nonce) ||
    BigEndian_u32(LogIndex) ||
    TxHash (32 bytes) ||
    FromAddress (20 bytes)
)
```

Internal nodes are standard Merkle parents:
```
InternalHash = SHA256(LeftHash || RightHash)
```

### 5.2. Pattern DSL Grammar
The pattern language follows Extended Backus-Naur Form (EBNF):

```ebnf
pattern    ::= sequence | quantified
sequence   ::= symbol ("->" symbol)*
quantified ::= symbol "{" min ["," max] "}"
symbol     ::= ALPHA (ALPHA | DIGIT | "_")*
min        ::= DIGIT+
max        ::= DIGIT+
ALPHA      ::= [a-zA-Z]
DIGIT      ::= [0-9]
```

### 5.3. P2P Message Formats
Messages are serialized using `bincode`.

**ThreatRule:**
| Field | Type | Description |
|-------|------|-------------|
| `pattern_name` | String | Human readable name |
| `pattern_expr` | String | DSL expression |
| `confidence` | f32 | 0.0 to 1.0 confidence required |
| `signature` | Secp256k1 | 65-byte Compact ECDSA over SHA256(Message) |

### 5.4. Proof Serialization
Proofs are compact binary blobs.
- **Structure**: `[Root (32b)][TargetHash (32b)][Index (u32)][Siblings (Vec<32b>)]`

## 6. Security Model

### 6.1. Trust Boundaries
- **Layer 0**: Trusted (Code correctness).
- **Layer 1**: Semi-Trusted (RPC Provider). A malicious RPC can omit logs, but cannot forge Merkle Proofs rooted in a known block hash.
- **Layer 2**: Untrusted. Peers may gossip false threats. Mitigated by PoBS (Reputation) and local re-verification.

### 6.2. Threat Mitigations
| Threat | Mitigation |
|--------|------------|
| **Sybil Attack** | Reputation requires proving past on-chain activity (Costly). |
| **Fake Proofs** | All proofs are cryptographically verifiable against the block header. |
| **RPC Ban** | Adaptive RPC Throttling prevents aggressive polling. |
| **Malicious Pattern** | DSL parser is strictly bounded (no recursion, max depth). |

## 7. Implementation Guide

### 7.1. Behavioral Merkle Tree (BMT) Structure - CURRENT
(Note: Placeholder for future Causal Merkle Tree structure)
```mermaid
graph TD
    Root[Root Hash] --> L[Hash 0-1]
    Root --> R[Hash 2-3]
    L --> Leaf0[Symbol A (Nonce 1)]
    L --> Leaf1[Symbol B (Nonce 1)]
    R --> Leaf2[Symbol C (Nonce 2)]
    R --> Leaf3[Symbol D (Nonce 5)]
```

### 7.2. Test Vectors
**Sepolia Block 10002322**
- **Symbols**: `Tf` (20), `Dep` (2), `Wdw` (1).
- **Correct Root**: `0x12dbccd3f68a2f13ca04e20a66e8ec90cb6c394a73ba405de6b6688b7073ca30`

### 7.3. Requirements
- **Language**: Rust (stable) 1.70+
- **Crypto**: `k256`, `sha2`
- **Network**: `libp2p` (gossipsub feature)

## 8. Extensions & Future Work

1.  **zk-Behavioral Proofs**: Wrapping SODS verification in a SNARK to prove behavioral compliance on-chain (L2 validation).
2.  **Cross-Chain Oracles**: Using P2P consensus to verify behavior on Chain A and report it to Chain B.
3.  **AI Pattern Discovery**: ML models analyzing BMT structures to find anomaly clusters automatically.

---
*End of Specification*
