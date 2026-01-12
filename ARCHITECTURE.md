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
3. **Social consensus**: Minimum 2/3 peers must agree on BMT root
4. **Local validation**: Proofs verified locally using sods-core

### Threat Mitigations

| Threat | Mitigation |
|--------|------------|
| Malicious peer sends fake proof | Signature verification + consensus |
| Peer spoofs another peer's identity | secp256k1 public key binding |
| RPC rate limiting | LRU cache + exponential backoff |
| Sybil attack (many fake peers) | Reputation tracking + fallback to RPC |
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
