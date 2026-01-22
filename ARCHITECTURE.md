# SODS Protocol — Architecture

This document describes the technical architecture, data flow, and trust model of the SODS Protocol implementation.

## Overview

SODS (Symbolic On-Demand Verification over Decentralized Summaries) enables trustless verification of blockchain behavioral claims using Merkle proofs and social consensus.

```
┌─────────────────────────────────────────────────────────────────┐
│                         USER / CLI                              │
│                      sods-cli (Layer 3)                         │
└──────────────────────────┬──────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     P2P NETWORK                                 │
│                   sods-p2p (Layer 2)                            │
│                                                                 │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐           │
│  │   Peer A    │   │   Peer B    │   │   Peer C    │           │
│  │  (signed)   │   │  (signed)   │   │  (signed)   │           │
│  └──────┬──────┘   └──────┬──────┘   └──────┬──────┘           │
│         │                 │                 │                   │
│         └────────────┬────┴─────────────────┘                   │
│                      │                                          │
│              Social Consensus                                   │
│           (2/3 majority required)                               │
└──────────────────────┬──────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│                   LOCAL VERIFIER                                │
│                sods-verifier (Layer 1)                          │
│                                                                 │
│  ┌─────────────────┐   ┌─────────────────┐                     │
│  │   LRU Cache     │◄──│   RPC Client    │                     │
│  │  (100 blocks)   │   │   (backoff)     │                     │
│  └─────────────────┘   └────────┬────────┘                     │
│                                 │                               │
└─────────────────────────────────┼───────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────┐
│                    SYMBOLIC CORE                                │
│                  sods-core (Layer 0)                            │
│                                                                 │
│  ┌───────────────┐   ┌───────────────┐   ┌───────────────┐    │
│  │   Dictionary  │   │      BMT      │   │    Proof      │    │
│  │   (symbols)   │   │    (tree)     │   │  (202 bytes)  │    │
│  └───────────────┘   └───────────────┘   └───────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
                         ┌───────────────┐
                         │   Ethereum    │
                         │   RPC Node    │
                         └───────────────┘
```

## Data Structures

### 1. **Behavioral Merkle Trees (BMT)**
Sorted binary Merkle trees containing behavioral symbols.
- **Leaves**: `H(symbol || metadata)`
- **Sorting**: Canonical ordering by block position (log index).
- **Goal**: Provide a unique, deterministic commitment to the block's behavior for simple presence verification.

### 2. **Causal Merkle Trees (CMT)**
Sorted by `(Origin, Nonce, Sequence)` to reconstruct coherent narratives.
- **Goal**: Prove that a sequence of events (e.g. `Tf -> Sw -> Tf`) was executed by a single actor in a specific order.
- **Verification**: Proofs must demonstrate contiguous nonces to be valid.

### 3. **Behavioral Symbols**
Standardized event representations (e.g., `Tf`, `Sw`) derived from raw logs, enriched with causality metadata (`tx_hash`, `nonce`).

## Layer Responsibilities

| Layer | Crate | Responsibility |
|-------|-------|----------------|
| 0 | sods-core | Symbol encoding, Merkle tree construction, proof generation |
| 1 | sods-verifier | RPC fetching, caching, local verification |
| 2 | sods-p2p | Peer discovery, signed message exchange, consensus |
| 3 | sods-cli | User interface, command parsing, output formatting |

## Trust Model

**Principle: Verify, Don't Trust**

1. **No trusted peers**: All peers are assumed potentially malicious
2. **Cryptographic verification**: Every response signed with secp256k1
3. **Adaptive Sybil Immunity**: Consensus based on **Proof-of-Behavioral-Stake (PoBS)**
4. **Local validation**: Proofs verified locally using sods-core

### Threat Mitigations

| Threat | Mitigation |
|--------|------------|
| Malicious peer sends fake proof | **Reputation Decay** + Weighted Consensus |
| Peer spoofs another peer's identity | secp256k1 public key binding |
| RPC rate limiting | LRU cache + exponential backoff |
| Sybil attack (many fake peers) | **PoBS**: Reputation requires valid work; decays over time |
| Man-in-the-middle | libp2p noise encryption |

## Data Flow

### Verify Request

```
1. User: sods verify Dep --block 10002322
2. CLI: Parse args, select mode (auto/p2p/rpc)
3. P2P Client: Connect to peers, send ProofRequest
4. P2P Peers: Each fetches logs via RPC (cached), builds BMT, signs response
5. P2P Client: Collect responses, verify signatures, evaluate consensus
6. CLI: Display result with confidence level
```

### Signature Verification

```
ProofResponse {
    proof_bytes: Vec<u8>,
    bmt_root: [u8; 32],
    success: bool,
    error: Option<String>,
    occurrences: usize,
    signature: Vec<u8>,   // 64 bytes ECDSA
    public_key: Vec<u8>,  // 33 bytes compressed
}

hash = SHA256(proof_bytes || bmt_root || success || error || occurrences)
valid = secp256k1.verify(hash, signature, public_key)
```

valid = secp256k1.verify(hash, signature, public_key)
```

## Decentralized Threat Intelligence Network

Uses **Gossipsub** protocol (`/sods/threats/1.0.0`) to propagate signed behavioral patterns.

### Security Model

- **No Central Authority**: Anyone can publish, but nodes only accept rules from keys they trust.
- **Signed Rules**: All `ThreatRule` messages must carry a valid ECDSA signature.
- **Validation**:
  1. **Syntax Check**: Pattern must be valid DSL.
  2. **Signature Check**: Must match `author_pubkey`.
  3. **Trust Check**: `author_pubkey` must carry in local `trusted_keys.json` (Managed via `sods threats add-key`).

```rust
struct ThreatRule {
    id: String,
    pattern: String,
    signature: Vec<u8>,
    author_pubkey: Vec<u8>, // Trusted?
}
```

## Version History

| Version | Changes |
|---------|---------|
| v0.1.0 | Initial implementation |
| v0.2.0 | LRU caching, exponential backoff, signed P2P, identify protocol |
| v1.0-beta | Production-ready release |

## Security Considerations

- Private keys are generated fresh on each peer startup (not persisted)
- No telemetry or external data collection
- All network traffic encrypted with libp2p noise
- Cache entries expire on process restart
