# SODS Protocol — RFC Draft v0.2
## Symbolic On-Demand Verification over Decentralized Summaries

**Status**: Experimental  
**Author**: LogicCrafter (logiccrafterdz@gmail.com)  
**Date**: January 7, 2026  
**Repository**: https://github.com/logiccrafter/sods-protocol  

---

## Abstract

This document specifies **SODS (Symbolic On-Demand Verification over Decentralized Summaries)**, an experimental protocol for **zero-trust, cost-effective behavioral verification** on public blockchains. SODS enables any user to verify whether a specific behavioral pattern occurred in a transaction history (e.g., `LP+ → LP-`, sandwich attack, flash loan sequence) **without fetching raw transaction data, logs, or traces**, and **without relying on centralized indexers or paid APIs**.

The protocol achieves this by encoding behavioral signatures as cryptographic commitments (Behavioral Merkle Trees, or BMTs) that are computed off-chain, broadcast via peer-to-peer consensus, and verified locally by light clients. SODS requires no Layer 1 protocol changes, no token incentives, and no persistent local storage. The protocol is designed for Ethereum and EVM-compatible blockchains.

---

## 1. Introduction

### 1.1. Motivation

Public blockchains are append-only, trustless ledgers. Yet reading them efficiently today requires:

- **Centralized indexers** (The Graph, Dune) — extracting historical data via proprietary APIs
- **Paid APIs** (Arkham, Nansen) — restricted data access behind paywalls
- **Full archive nodes** (Geth, Erigon) — expensive to operate, multiple terabytes of storage

This contradicts the foundational principle of blockchain:  
> **"Don't trust. Verify."**

Verification of on-chain behavior should be as economical as appending new data—not orders of magnitude more expensive.

### 1.2. Core Insight

Blockchain behavior is **deterministic**: given a block's logs and transactions, whether a pattern occurred (e.g., two liquidity events by the same address) is computable without re-executing all state. If transaction data can be committed via Merkle trees, **so can behavioral summaries**.

This document introduces the **Behavioral Merkle Tree (BMT)**—a lightweight, off-chain data structure where:
- **Leaves** are symbolic tokens (canonical abbreviations like `Tf` for Transfer, `Sw` for Swap, `LP+` for liquidity addition)
- **Branches** follow standard Merkle-tree hashing
- **Root** is deterministic for each block, verifiable by any light client independently

A BMT root can be computed by independent light clients, advertised via P2P, and verified cryptographically—enabling **true on-demand behavioral verification without centralized intermediaries**.

### 1.3. Scope and Applicability

This specification applies to:

- **Ethereum mainnet and EVM-compatible chains** (Polygon, Arbitrum, Optimism)
- **Historical behavior queries** (did event X occur in block N?)
- **Read-only verification** (no state mutations, no transactions required)
- **Light client deployments** (no full node necessary)

This specification does **not** address:

- Real-time (mempool) behavior prediction
- Orderings of events within a block (only aggregate occurrence per block)
- Non-EVM blockchains (though the concept is generalizable)
- Incentive mechanisms or token economies
- Private/confidential blockchains

### 1.4. Structure of This Document

This RFC is organized as follows:

- **Section 2**: Protocol overview and architecture (4-layer model)
- **Section 3**: Symbolic encoding rules and canonical symbol set
- **Section 4**: Behavioral Merkle Tree (BMT) construction algorithm
- **Section 5**: P2P claim network topology and consensus mechanism
- **Section 6**: On-demand verification algorithm and confidence scoring
- **Section 7**: Security considerations and threat model
- **Section 8**: Implementation roadmap and examples
- **Section 9**: Conformance Tests
- **Section 10**: References

---

## 2. Protocol Overview

SODS operates in four conceptual layers:

| Layer | Component | Responsibility | Implementation |
|-------|-----------|-----------------|-----------------|
| **0** | Symbolic Encoder | Parse EVM logs → canonical symbolic tokens | Deterministic state machine |
| **1** | Behavioral Merkle Tree (BMT) | Aggregate tokens per block into Merkle root | Off-chain, computed independently by light clients |
| **2** | P2P Claim Network | Light clients advertise `(block, BMT_root, signature)` tuples | libp2p-based gossip (Ethereum-like) |
| **3** | On-Demand Verifier | Query P2P network for proofs; verify locally; compute confidence | Client library (CLI + SDK) |

### 2.1. Query Flow Example

```
User query: "Did LP+ → LP- occur in block N?"
       ↓
On-Demand Verifier initiates query
       ↓
Verifier requests proofs from ≥3 independent light clients (P2P)
       ↓
Each client responds with:
   • Symbol (e.g., "LP+")
   • Merkle proof (path from leaf to root)
   • Claimed BMT root
   • Client signature (for reputation tracking)
       ↓
Verifier validates:
   1. Each proof's cryptographic validity
   2. Cross-client consensus on BMT root (≥2/3 agreement)
   3. Pattern match: LP+ present, then LP- present in same block
       ↓
Output (JSON):
{
  "pattern": "LP+ → LP-",
  "verified": true,
  "confidence": 0.82,
  "block_number": N,
  "num_agreeing_nodes": 3,
  "missing_context": ["internal_call_order"],
  "recommendation": "High confidence; pattern detected"
}
```

---

## 3. Symbolic Encoding (Layer 0)

### 3.1. Requirement Language

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119 [RFC2119].

### 3.2. Symbol Format Definition

A symbolic token is a UTF-8 string representing an atomic blockchain operation:

```
symbol := <OPCODE> [ "@" <CONTEXT> ] [ "#d" <OFFSET> ]

OPCODE  := [A-Z0-9]+              ; e.g., "Tf", "Sw", "LP+", "LP-"
CONTEXT := [A-Za-z0-9]+           ; e.g., "U2" (Uniswap V2), "Aave"
OFFSET  := [0-9]+                 ; relative block offset (Δ)
```

Examples:
- `Tf` — ERC20 Transfer
- `Sw@U2` — Swap on Uniswap V2
- `LP+#d5` — Add liquidity with 5-block relative offset

### 3.3. Core Symbol Set (Atomic Only)

The Core Set **MUST** contain **only atomic, log-derived symbols**.  
Composite patterns (e.g., sandwich) are **NOT part of Layer 0**.

| Type | Example | Layer | Responsibility |
|------|---------|-------|----------------|
| **Atomic Symbol** | `Sw`, `Tf`, `LP+` | Layer 0 | Extracted directly from logs |
| **Derived Pattern** | `S` (Sandwich), `RugPull` | Layer 3 Plugin | Computed by verifier from atomic symbols |

> This separation ensures:
> - Layer 0 remains minimal and consensus-safe  
> - Layer 3 can evolve independently (e.g., ML-based pattern detection)

**Core Immutable Registry (Atomic only):**

| Symbol | Event Signature | EVM Topic | Canonical URI | Semantics |
|--------|-----------------|-----------|---------------|-----------|
| `Tf` | `Transfer(address indexed from, address indexed to, uint256 value)` | `0xddf252ad...` (ERC20) | `sods://sym/erc20/transfer` | Token transfer |
| `M` | `Mint(address indexed to, uint256 value)` | `0x0f6798a5...` (ERC20) | `sods://sym/erc20/mint` | Token minting |
| `B` | `Burn(address indexed from, uint256 value)` | (variable) | `sods://sym/erc20/burn` | Token burning |
| `A` | `Approval(address indexed owner, address indexed spender, uint256 value)` | `0x8c5be1e5...` (ERC20) | `sods://sym/erc20/approval` | Approval grant |
| `Sw` | `Swap(address indexed sender, uint256 amount0In, ...)` | `0xd78ad95f...` (Uniswap V2) | `sods://sym/swap/uniswap_v2` | Token swap |
| `LP+` | `Mint(address indexed sender, uint256 amount0, uint256 amount1)` | (Uniswap V2 pair) | `sods://sym/lp/addliquidity` | Add liquidity |
| `LP-` | `Burn(address indexed sender, uint256 amount0, uint256 amount1)` | (Uniswap V2 pair) | `sods://sym/lp/removeliquidity` | Remove liquidity |
| `F` | `FlashLoan(address indexed target, uint256 amount, uint256 fee)` | (Aave/custom) | `sods://sym/flash/loan` | Flash loan borrow |

### 3.4. Pattern Plugins (Layer 3)

Verifiers **MAY** load pattern plugins (e.g., `mev.sandwich`, `defi.rugpull`) that define:

```json
{
  "pattern_id": "S",
  "uri": "sods://pattern/mev/sandwich/v1",
  "requires": ["Sw"],
  "logic": "sequence of 3 Sw by addresses [A, B, A] within same block"
}
```

- Plugins are **not** part of consensus — only for local reasoning.

### 3.5. Symbol Encoding Algorithm

For each event log in a block:

1. Extract `topics[0]` (event signature hash)
2. Compare against Core Symbol Set (lookup by EVM topic)
3. If match found → emit symbolic token
4. If no match → skip log (extensions may define additional symbols via packs)

**Pseudocode:**

```python
def encode_block(block):
    """
    Encode all logs in a block to symbolic tokens.
    
    Args:
        block: Block object with logs array
        
    Returns:
        List of (symbol, block_index, log_index, metadata) tuples,
        sorted by: (1) block_index ↑, (2) log_index ↑, (3) symbol lex ↑.
    """
    symbols = []
    
    # Iterate through transactions and logs to preserve order context
    for tx_idx, tx in enumerate(block.transactions):
        for log_idx, log in enumerate(tx.logs):
            topic = log.topics[0]
            symbol = SYMBOL_REGISTRY.lookup(topic)
            
            if symbol:
                # Extract optional metadata (address, amount, etc.)
                metadata = {
                    'from_address': log.address,
                    'topics': log.topics[1:],
                    'data': log.data
                }
                symbols.append((
                    symbol,
                    tx_idx,      # ← block_index = position in block
                    log_idx,     # ← log_index = position in tx
                    metadata
                ))
    
    # Deterministic sort (critical for BMT reproducibility)
    symbols.sort(key=lambda x: (x[1], x[2], x[0]))  # tx_idx → log_idx → symbol
    return symbols
```

### 3.6. Assumptions & Trust Boundaries

SODS is a **Verification Protocol**, not a **Data Availability Protocol**.
It relies on explicit assumptions — it does not hide them, but declares them:

| Layer | Trust Assumption | Mitigation | Future Integration |
|-------|------------------|------------|--------------------|
| **Log Availability** | External RPC or Portal Network must provide logs for block `N`. | Verifier MAY cross-validate logs via multiple providers (e.g., Infura + Alchemy + local node). | **Portal Network (v0.4+)**: Use `History Network` to fetch logs without trusting any single node. |
| **Log Integrity** | Logs received must be authentic (i.e., match `block.receiptsRoot`). | Verifier computes `receiptsRoot` locally if receipts available. **Warning**: Light clients without full sync cannot verify this (must trust RPC or Portal Network). | **EIP-4844 DAS**: Leverage data availability sampling for cryptographic log integrity. |
| **Node Identity** | Peer addresses (`signer`) are Ethereum addresses — but not economically secured. | Reputation built on historical accuracy (see §5.4). | **EIP-7251**: Integrate with light client stake delegation for stronger sybil resistance. |

> **Philosophical Clarification**:  
> *"SODS does not eliminate trust — it makes trust explicit, quantifiable, and optional to verify."*

---

## 4. Behavioral Merkle Tree (BMT) — Layer 1

### 4.1. BMT Construction Algorithm

For each block B, construct a Merkle tree as follows:

**Input**: Block B (containing transactions and logs)  
**Output**: `BMT_root` (32-byte hash), `BMT_tree` (proof structure for queries)

**Steps**:

1. **Encode**: Extract all symbolic tokens from block B's logs (§3.5) → list of `(symbol, block_index, log_index, metadata)`
2. **Sort**: Sort by primary key `(block_index, log_index)`, using `symbol` as tie-breaker.
3. **Hash leaves**: Compute leaf hashes based on selected **Leaf Mode** (§4.1.1).
4. **Build tree**: Construct Merkle tree:
   ```
   leaves = [leaf_1, leaf_2, ..., leaf_n]
   tree = merkle_build(leaves)
   BMT_root = tree.root
   ```
5. **Store proofs**: For each leaf, store Merkle proof (path to root) for later query

**Pseudocode**:

```python
import hashlib
import cbor2
from merkle import MerkleTree

def build_bmt(block, leaf_mode="minimal"):
    """
    Build Behavioral Merkle Tree for a block.
    
    Args:
        block: Block object
        leaf_mode: "minimal" or "full"
        
    Returns:
        {
            'root': bytes32,
            'tree': MerkleTree object,
            'symbols': List[dict]
        }
    """
    # Step 1: Encode symbols
    # formatted as: (symbol, block_idx, log_idx, metadata)
    symbols = encode_block(block)
    
    # Step 2: Sort by primary key (block_index, log_index)
    # symbol lex order is tie-breaker
    symbols.sort(key=lambda x: (x[1], x[2], x[0]))
    
    # Step 3: Hash leaves
    leaves = []
    symbols_with_proofs = []
    
    for symbol, b_idx, l_idx, metadata in symbols:
        if leaf_mode == "full":
            # Canonical CBOR serialization
            meta_bytes = cbor2.dumps(metadata, canonical=True)
            leaf_data = symbol.encode() + meta_bytes
        else:
            # Minimal mode (symbol only)
            leaf_data = symbol.encode()
            
        leaf = hashlib.sha256(leaf_data).digest()
        leaves.append(leaf)
        
        symbols_with_proofs.append({
            'symbol': symbol,
            'indexes': (b_idx, l_idx),
            'metadata': metadata,
            'leaf': leaf
        })
    
    # Step 4: Build Merkle tree
    if not leaves:
        bmt_root = hashlib.sha256(b'').digest()
        return {'root': bmt_root, 'tree': None, 'symbols': []}
    
    tree = MerkleTree(leaves)
    
    # Step 5: Store proofs
    for i, item in enumerate(symbols_with_proofs):
        item['proof'] = tree.get_proof(i)
        item['index'] = i
    
    return {
        'root': tree.root,
        'tree': tree,
        'symbols': symbols_with_proofs
    }
```

### 4.1.1. Leaf Composition Modes

To balance verification strength and computational cost, SODS defines two **canonical leaf modes**:

| Mode | Leaf Construction | Use Case | Properties |
|------|-------------------|----------|------------|
| **BMT-Minimal** | `leaf = SHA256(symbol)` | Fast, lightweight verification (e.g., CLI scanning) | ✅ Size-efficient (32B/leaf)<br>⚠️ Cannot verify metadata (address, amounts)<br>⚠️ Vulnerable to symbol spoofing without social consensus |
| **BMT-Full** | `leaf = SHA256(symbol || canonical_metadata)` | High-assurance verification (e.g., audit tools) | ✅ Cryptographically binds symbol to event details<br>⚠️ Larger proofs (~130B/leaf)<br>⚠️ Requires strict canonical encoding (§4.4) |

> **Implementation Note**:  
> All conformant clients **MUST** support *at least* BMT-Minimal.  
> BMT-Full is OPTIONAL but RECOMMENDED for security-critical applications.  
> The claimed mode **MUST** be included in `BMT_CLAIM` messages (see §5.2).

### 4.2. Merkle Proof Structure

For each symbol in a block, the Merkle proof consists of:

```python
{
    'symbol': str,              # e.g., "LP+"
    'leaf_hash': bytes32,       # SHA256(symbol || metadata)
    'index': int,               # Position in sorted symbol list
    'proof': [bytes32],         # Sibling hashes (path to root)
    'metadata': {
        'address': address,     # EVM address
        'topics': [bytes32],    # Log topics (from-to, amounts, etc.)
        'data': bytes           # Log data
    }
}
```

**Tree Construction Rules**:  
- Leaf indices are 0-based and sorted as per §4.4.2  
- Proof path is ordered **from leaf → root**  
- At each level:  
  - If index is **even**, sibling is **right child** → combine: `hash = H(current || sibling)`  
  - If index is **odd**, sibling is **left child** → combine: `hash = H(sibling || current)`  
- Empty block (no symbols): `BMT_root = SHA256(b'')`  
- Single leaf: `BMT_root = leaf_hash`

**Verification pseudocode**:

```python
def verify_merkle_proof(symbol_obj, bmt_root):
    """
    Verify a Merkle proof against a claimed BMT root.
    
    Args:
        symbol_obj: {symbol, leaf_hash, index, proof, metadata}
        bmt_root: claimed root hash
        
    Returns:
        bool: True if proof is valid
    """
    hash_val = symbol_obj['leaf_hash']
    
    # Traverse proof from leaf to root
    for sibling_hash in symbol_obj['proof']:
        # Determine left/right based on index
        if symbol_obj['index'] % 2 == 0:
            combined = hash_val + sibling_hash
        else:
            combined = sibling_hash + hash_val
        
        hash_val = hashlib.sha256(combined).digest()
        symbol_obj['index'] //= 2
    
    return hash_val == bmt_root
```

### 4.3. Properties of BMT

- **Deterministic**: Same block input → same root (across all independent implementations)
- **Commitment-based**: Small root (32 bytes) commits to all behaviors in block
- **Append-only**: No re-ordering; symbols added in log order
- **Lightweight**: 1 symbol ≈ 32 bytes (hash) + ~100 bytes (metadata); total per block ≈ 50KB (typical)
- **Off-chain**: Never written to L1 state; computed and shared via P2P

### 4.4. Canonical Encoding Rules

To ensure deterministic `BMT_root` across all implementations, the following encoding rules **MUST** be followed:

#### 4.4.1. Metadata Serialization

All metadata (address, topics, data) **MUST** be serialized using **strict CBOR (RFC 8949)** with:
- Canonical encoding (deterministic map key order, no extra spaces)
- Unsigned integers encoded in minimal form
- Byte strings encoded as `0x`-prefixed hex (UTF-8)

Example:
Example:
```json
{
  "address": "0x742d35Cc6634C0532925a3b8D4C4E8C5E1771d1b",
  "topics": [
    "0x000000000000000000000000742d35cc6634c0532925a3b8d4c4e8c5e1771d1b",
    "0x00000000000000000000000068b32a4e5b89a3f4c5d6e7f8a9b0c1d2e3f4a5b6"
  ],
  "data": "0x00000000000000000000000000000000000000000000000b1a2bc2ec50000"
}
```
Canonical CBOR hex (truncated):
`a367616464726573735814742d35cc...646174615820000000...`

#### 4.4.2. Log Ordering

Symbols **MUST** be sorted by:
1. `block_index` (ascending) — position in block
2. `log_index` (ascending) — position within transaction
3. `symbol` (lexicographic) — *only as tie-breaker* when (1) and (2) are equal

> This guarantees **deterministic replay of event order** without relying on unstable metadata.

#### 4.4.3. ABI Normalization

- `address` fields: left-padded to 20 bytes
- `uint256` values: big-endian, 32 bytes
- Indexed topics: hashed only if type is `string`/`bytes`; else raw value

#### 4.4.4. Empty/Missing Fields

- If a field is optional and missing (e.g., `topics[2]`), encode as `null` (CBOR major type 7, value 22)
- Never omit keys in maps

> **Conformance Test (BMT-005)**:  
> Two independent implementations of `build_bmt()` **MUST** produce identical `BMT_root` for the same block.

---

## 5. P2P Claim Network — Layer 2

### 5.1. Network Architecture

The P2P Claim Network is a libp2p-based gossip protocol, similar to Ethereum's DevP2P but specialized for broadcasting BMT claims. It is designed to extend the light client model by integrating with emerging standards:

- **EIP-4844 (Proto-Danksharding)**: Leveraging data availability blobs for cost-effective historical log access.
- **EIP-7002**: Supporting execution layer triggerable exits, enhancing the trust model for validator proxies.
- **Ethereum Portal Network**: Using the Portal Network for decentralized, lightweight retrieval of historical data.

#### Node Types

| Type | Role | Requirements | Trust Model |
|------|------|--------------|-------------|
| **Light Client** | Computes BMTs independently from RPC data | RPC access to EVM node | Self-verified (computes locally) |
| **Validator Proxy** | Bridges full node to light clients (optional) | Full node access | Optional delegation |
| **Cache Node** | Stores BMT roots for recent blocks (last 256, ~2 weeks) | RPC + disk (< 100MB) | Reciprocal peer trust |

### 5.2. Message Types

All P2P messages are JSON (for clarity in spec; implementations may use binary serialization):

#### Message: BMT Claim

```json
{
  "type": "BMT_CLAIM",
  "version": 1,
  "protocol_version": "0.3",
  "block_number": 20123456,
  "block_hash": "0xabc...def",
  "bmt_root": "0x123...abc",
  "leaf_mode": "minimal",
  "timestamp": 1704635000,
  "signer": "0xNode1Address",
  "signature": "0xSignature...",
  "num_symbols": 42
}
```

**Fields**:
- `block_number`, `block_hash`: Identify block
- `bmt_root`: Claimed root
- `signer`: Address of node making claim
- `signature`: ECDSA signature over `(block_number, bmt_root, timestamp)` with signer's private key
- `num_symbols`: Count of symbols in block (for sanity check)

#### Message: Merkle Proof Request

```json
{
  "type": "PROOF_REQUEST",
  "version": 1,
  "block_number": 20123456,
  "symbol": "LP+",
  "request_id": "uuid-12345"
}
```

#### Message: Merkle Proof Response

```json
{
  "type": "PROOF_RESPONSE",
  "version": 1,
  "request_id": "uuid-12345",
  "block_number": 20123456,
  "bmt_root": "0x123...abc",
  "symbol": "LP+",
  "leaf_hash": "0xabc...123",
  "index": 17,
  "proof": ["0x...", "0x...", ...],
  "metadata": {
    "address": "0xDeployer",
    "topics": [...],
    "data": "0x..."
  },
  "signature": "0xNodeSignature..."
}
```

### 5.3. Social Consensus Mechanism

When a verifier wants to confirm a BMT root for block N:

**Algorithm**:

```python
def consensus_query(block_num, target_symbol=None):
    """
    Query ≥3 independent light clients for consensus on BMT root.
    
    Args:
        block_num: Block to query
        target_symbol: Optional symbol to focus on (None = all symbols)
        
    Returns:
        {
            'consensus_root': bytes32 or None,
            'num_agreeing': int,
            'disagreements': List[bytes32],
            'confidence': float
        }
    """
    # 1. Sample ≥3 random peer nodes
    peers = sample_random_peers(count=5, exclude_known_malicious=True)
    
    # 2. Send BMT_CLAIM requests
    responses = {}
    for peer in peers:
        try:
            response = peer.request_bmt_claim(block_num)
            responses[peer.id] = response
        except Timeout:
            continue
    
    # 3. Tally roots
    root_count = {}
    for peer_id, response in responses.items():
        root = response['bmt_root']
        if root not in root_count:
            root_count[root] = []
        root_count[root].append(peer_id)
    
    # 4. Determine consensus
    if not root_count:
        return {
            'consensus_root': None,
            'num_agreeing': 0,
            'disagreements': [],
            'confidence': 0.0
        }
    
    # Consensus = most common root with ≥2/3 agreement
    sorted_roots = sorted(
        root_count.items(),
        key=lambda x: len(x[1]),
        reverse=True
    )
    consensus_root, agreeing_peers = sorted_roots[0]
    disagreeing = [r for r, _ in sorted_roots[1:]]
    
    if len(agreeing_peers) >= len(responses) * 2 / 3:
        confidence = len(agreeing_peers) / len(responses)
        return {
            'consensus_root': consensus_root,
            'num_agreeing': len(agreeing_peers),
            'disagreements': disagreeing,
            'confidence': confidence
        }
    else:
        return {
            'consensus_root': None,
            'num_agreeing': 0,
            'disagreements': [r for r, _ in sorted_roots],
            'confidence': 0.0,
            'note': 'No clear consensus; network split'
        }
```

**Consensus Rules**:

1. Query ≥3 independent nodes (different geographic regions if possible)
2. If ≥2/3 agree on same root → **Accept as consensus**
3. If disagreement → Flag as `unreliable`, mark outlier nodes, retry with different peers
4. If 2+ competing roots have equal agreement → Network state uncertain; return low confidence

**Consensus Threshold Rule**:
A claim is accepted if the number of **agreeing peers ≥ ⌈2n/3⌉**, where *n* = total responding peers.

Examples:
- 3 peers → need **2** agreeing (66.7% ≥ 66.7%)
- 4 peers → need **3** agreeing (75% ≥ 66.7%)
- 5 peers → need **4** agreeing (80% ≥ 66.7%)
- 6 peers → need **4** agreeing (66.7% ≥ 66.7%)

⌈2n/3⌉ ensures **Byzantine fault tolerance** (≤ f = ⌊(n−1)/3⌋ malicious nodes tolerated).

> **Note**: This is lightweight, requires no on-chain state, and is fully optional (fallback to simple 2/3 if disabled).
```

> **Note**: Section 5.4 has been moved to [§8.4 Reputation-Based Consensus](#84-reputation-based-consensus-planned-for-v04).

---

## 6. On-Demand Verification (Layer 3)

### 6.1. Query Language and Syntax

```
pattern := symbol [ "→" symbol ]* [ "within" number "blocks" ]
context := [ "from" address ] [ "to" address ] [ "on" chain ]
query   := "verify" pattern [ context ]

Examples:
  - "verify LP+"
  - "verify LP+ → LP- within 5 blocks"
  - "verify Sw from 0x123... to 0x456... on eth"
  - "verify Sw → Tf within 10 blocks"
```

**Verification is always binary** (`verified: true/false`). Confidence guides human judgment.

```python
def verify_pattern(pattern_str, block_num, verifier_context):
    """
    Verify a behavioral pattern in a specific block.
    
    Args:
        pattern_str: Query string, e.g., "LP+ → LP-"
        block_num: Block number to verify
        verifier_context: {from_addr, to_addr, chain}
        
    Returns:
        {
            'pattern': str,
            'verified': bool,
            'confidence': float [0.0, 1.0],
            'block_number': int,
            'num_agreeing_nodes': int,
            'missing_context': List[str],
        {
            'pattern': str,
            'verified': bool,
            'confidence': float [0.0, 1.0],
            'evidence_type': str,  # "cryptographic", "heuristic", or "external"
            'block_number': int,
            'num_agreeing_nodes': int,
            'missing_context': List[str],
            'details': {...}
        }
    """
    # 1. Parse pattern
    symbols = parse_pattern(pattern_str)
    block_offset = extract_offset(pattern_str)
    
    # 2. Query consensus for block
    consensus = consensus_query(block_num)
    if not consensus['consensus_root']:
        return {
            'verified': False,
            'confidence': 0.0,
            'error': 'No consensus on BMT root'
        }
    
    consensus_root = consensus['consensus_root']
    
    # 3. Request proofs for each symbol in pattern
    proofs = {}
    for symbol in symbols:
        proof_response = request_proof(
            block_num,
            symbol,
            consensus_root
        )
        if proof_response:
            proofs[symbol] = proof_response
        else:
            proofs[symbol] = None
    
    # 4. Verify each proof
    all_valid = True
    for symbol, proof in proofs.items():
        if proof is None:
            all_valid = False
            break
        
        is_valid = verify_merkle_proof(proof, consensus_root)
        if not is_valid:
            all_valid = False
            break
    
    # 5. Check pattern match
    if not all_valid:
        return {
            'verified': False,
            'confidence': 0.0,
            'error': 'Merkle proof verification failed'
        }
    
    # Pattern match: all symbols present?
    pattern_found = all(s in proofs for s in symbols)
    
    # 6. Compute confidence score
    confidence = compute_confidence(
        pattern_found,
        consensus['num_agreeing'],
        len(proofs),
        verifier_context
    )
    
    return {
        'pattern': pattern_str,
        'verified': pattern_found,
        'confidence': confidence,
        'block_number': block_num,
        'num_agreeing_nodes': consensus['num_agreeing'],
        'missing_context': infer_missing_context(proofs),
        'details': {
            'consensus_root': consensus_root,
            'proofs': {s: p for s, p in proofs.items() if p}
        }
    }

> **Note on Confidence**: The confidence score is *epistemic*, not probabilistic. A value of `0.82` means *"the evidence is strong"* (based on node agreement and data coverage), **not** *"82% chance the pattern occurred."* Verification is binary (`verified: true/false`); confidence guides human judgment.

def compute_confidence(pattern_found, num_nodes, num_symbols, context):
    """
    Confidence scoring function.
    
    Factors (out of 100):
      + 30 if ≥3 nodes agree on BMT root
      + 20 if all symbols in pattern verified
      + 15 if context (address, chain) confirmed
      + 10 if block age < 7 days (lower risk of fork)
      - 20 if pattern not found
      - 15 if conflicting proofs exist
    """
    score = 0.0
    
    if num_nodes >= 3:
        score += 30.0
    elif num_nodes == 2:
        score += 15.0
    
    if pattern_found:
        score += 20.0
        score += min(15.0, num_symbols * 5.0)  # Bonus for multiple symbols
    else:
        score -= 20.0
    
    if context.get('from_addr') and context.get('to_addr'):
        score += 15.0
    
    # Normalize to [0, 1]
    confidence = max(0.0, min(1.0, score / 100.0))
    return confidence
```

### 6.2.1. Interpretation of Confidence

**Epistemic, Not Probabilistic**

The `confidence` field is an **epistemic score** — it quantifies *how much evidence supports the claim*, not the *probability the claim is true*.

- A confidence of `0.82` does *not* mean "82% chance pattern occurred."
- It means: *"Given the evidence (3 agreeing nodes, full metadata, known addresses), this claim is strongly supported — but not mathematically proven."*

**Verification is always binary** (`verified: true/false`). Confidence guides human judgment.

---

## 7. Security Considerations

### 7.1. Threat Model

| Threat | Attacker Goal | Mitigation | Residual Risk |
|--------|---------------|-----------|---------------|
| **Malicious Light Client** | Serve false BMT root to user | Social consensus (≥2/3 agreement required) | Low if ≥3 independent nodes |
| **Eclipse Attack** | Isolate user to controlled peer set | Random peer selection; reputation cache | Medium (requires active network separation) |
| **Sybil Attack** | Create many fake nodes to achieve quorum | Proof-of-Work or reputation system (future) | Medium (currently mitigated by random sampling) |
| **Chain Reorganization** | Force user to verify against orphaned block | Verifier queries consensus on block hash before BMT | Low if block age > 12 hours (Ethereum finality) |
| **Symbol Collision** | Forge false symbol by manipulating logs | Canonical URIs + immutable Core Symbol Set | Very Low (deterministic hash-based matching) |
| **Replay Attack** | Reuse old proof from different block | Proofs bound to specific `block_number` + `block_hash` | Very Low |

### 7.2. Privacy Considerations

**Data Leakage**:
- Queries are **unlinkable** (no session IDs; each query independent)
- Only **symbol presence** leaked; no raw addresses/amounts
- P2P network may infer query interest (acceptable for public blockchain data)

**Mitigation**:
- Use Tor or VPN if hiding query pattern is critical
- Query multiple irrelevant symbols to obfuscate true interest (future: anonymous query bundles)

### 7.3. Cryptographic Assumptions

SODS relies on:
- **SHA-256** for Merkle tree hashing (industry standard; collision resistance assumed)
- **ECDSA** for node signatures (Ethereum secp256k1; forgeability < 2^-128 with 256-bit keys)
- **Random peer sampling** for eclipse resistance (cryptographic randomness required)

No post-quantum assumptions are made; migration to post-quantum hashes (SHA-3, BLAKE3) is straightforward.

### 7.4. Conformance to Security Best Practices

This specification follows RFC 3552 [RFC3552] (Guidelines for Writing RFC Text on Security Considerations):

- **Threat model explicitly defined** (§7.1)
- **Mitigations described** for each threat
- **Residual risks noted**
- **Assumptions documented** (cryptographic, behavioral, network)

---

## 8. Implementation Status and Roadmap

### 8.1. Current Status

**Status**: Pre-Alpha research specification  
**Maturity**: Experimental; ready for community feedback and POC testing  
**Reference Implementation**: Planned (Rust + JavaScript)

### 8.2. Roadmap

| Phase | Timeline | Deliverable | Success Criteria |
|-------|----------|-------------|------------------|
| **PoC (Phase 1)** | Q1 2026 | CLI + light client on Sepolia testnet | Verify 5 core symbols; ≥3 peers report same root |
| **Testnet (Phase 2)** | Q2 2026 | P2P network (10+ nodes) on Goerli | Consensus query succeeds; no Byzantine failures |
| **Mainnet Lite (Phase 3)** | Q3 2026 | Core symbols on Ethereum mainnet | Historical queries verified; <500ms latency |
| **Mainnet Lite (Phase 3)** | Q3 2026 | Core symbols on Ethereum mainnet | Historical queries verified; <500ms latency |
| **Extensions (Phase 4)** | 2027+ | MEV, NFT, cross-chain symbol packs | Community-driven extension process |

### 8.4. Reputation-Based Consensus (Planned for v0.4)

*Note: The reputation system described in v0.2 draft is deferred to allow for:*
- *Formal Byzantine fault tolerance analysis*
- *Simulation-based parameter tuning (e.g., decay rate, blacklist threshold)*
- *Community feedback on sybil-resistance tradeoffs*

Proposed features include Accuracy Scores, Weighted Voting, and Reputation Epochs (see v0.2 archive for details).

### 8.3. Example Workflow (End-to-End)

**Scenario**: Detect a possible sandwich attack in block `20123456`.

**User command**:
```bash
sods-cli verify \
  --pattern "Sw → Sw → Sw" \
  --block 20123456 \
  --chain eth \
  --confidence-threshold 0.8
```

**Output**:
```json
{
  "pattern": "Sw → Sw → Sw",
  "verified": true,
  "confidence": 0.87,
  "evidence_type": "heuristic",
  "block_number": 20123456,
  "block_hash": "0xabc...def",
  "num_agreeing_nodes": 3,
  "consensus_root": "0x123...abc",
  "symbols_found": {
    "Sw": [
      {
        "index": 5,
        "address": "0xMEV_Bot",
        "topics": ["0xSender", "0xAmount0", "0xAmount1"],
        "timestamp_ms": 1704635012500
      },
      {
        "index": 8,
        "address": "0xVictim",
        "timestamp_ms": 1704635012501
      },
      {
        "index": 11,
        "address": "0xMEV_Bot",
        "timestamp_ms": 1704635012502
      }
    ]
  },
  "missing_context": [
    "internal_call_order (requires trace data)",
    "slippage impact (requires price feeds)"
  ],
  "note": "This detection is symbolic (sequence of Sw events). Economic impact verification (e.g., price slippage, profit extraction) is out of SODS scope and requires external data (price feeds, state diffs).",
  "recommendation": "High confidence sandwich attack detected; recommend review of transaction details via archive node or block explorer for full analysis."
}
```

---

## 9. Conformance Tests (Draft Proposal)

To ensure interoperability between SODS clients, the following conformance tests MUST be passed by any implementation.

| Test ID | Description | Expected Outcome |
|---------|-------------|------------------|
| `BMT-001` | Empty block (no logs) | `root = SHA256("")` (or defined empty hash) |
| `SYM-002` | Block with Transfer + Swap | `symbols = ["Sw", "Tf"]` (sorted) |
| `CON-003` | 2 agreeing peers + 1 disagreeing | `consensus = agreeing root` |
| `VER-004` | Verify `LP+` with valid proof | `verified: true` |
| `BMT-005` | Canonical CBOR encoding (metadata) | Identical roots across implementations |
| `BMT-006` | Empty metadata fields | Encoded as CBOR `null` (major type 7, value 22) |
| `SYM-007` | Symbol case sensitivity | `LP+` ≠ `lp+` (case-sensitive) |
| `CON-008` | Byzantine peer (malicious root) | Consensus rejects if > ⌊(n−1)/3⌋ malicious |
| `VER-010` | Merkle proof with wrong index | `verify_merkle_proof()` returns `false` |
| `VER-011` | Proof with tampered sibling hash | Verification fails |
| `REP-012` | Reputation decay simulation (30 days)| Score approaches 0.5 if no updates |
| `SYS-013` | Chain Reorg Handling | Verifier invalidates result if blockhash changes |

*Note: These tests should be strictly implemented in future versions, potentially as a `pytest` suite.*

---

## 10. References

### 9.1. Normative References

[RFC2119]  
Bradner, S., "Key words for use in RFCs to Indicate Requirement Levels", BCP 14, RFC 2119, March 1997, https://www.rfc-editor.org/info/rfc2119.

[RFC3629]  
Yergeau, F., "UTF-8, a transformation format of ISO 10646", STD 63, RFC 3629, November 2003, https://www.rfc-editor.org/info/rfc3629.

[FIPS180-4]  
National Institute of Standards and Technology, "Secure Hash Standard (SHS)", FIPS 180-4, August 2015, https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf.

[SECP256K1]  
Standards for Efficient Cryptography, "SEC 2: Recommended Elliptic Curve Domain Parameters", SEC 2 v2.0, January 2010, https://www.secg.org/sec2-v0.3.pdf.

### 9.2. Informative References

[RFC7322]  
Flanagan, H. and S. Ginoza, "RFC Style Guide", RFC 7322, September 2014, https://www.rfc-editor.org/info/rfc7322.

[RFC3552]  
Rescorla, E. and B. Korver, "Guidelines for Writing RFC Text on Security Considerations", BCP 72, RFC 3552, July 2003, https://www.rfc-editor.org/info/bcp72.

[RFC5741]  
Daigle, L. (Ed.), Kolkman, O. (Ed.), and IAB, "RFC Streams, Headers, and Boilerplates", RFC 5741, December 2009, https://www.rfc-editor.org/info/rfc5741.

[EIP-1]  
Martin, G., "EIP-1: EIP Purpose and Guidelines", Ethereum Improvement Proposals, https://eips.ethereum.org/EIPS/eip-1.

[ETHEREUM-YELLOW]  
Wood, G., "Ethereum: A Secure Decentralised Generalised Transaction Ledger", Ethereum Project Yellow Paper, https://ethereum.org/en/whitepaper/.

[LIBP2P]  
Protocol Labs, "libp2p: A Modular Peer-to-Peer Networking Stack", https://docs.libp2p.io/.

[MERKLE-TREE-WIKI]  
Wikipedia, "Merkle Tree", https://en.wikipedia.org/wiki/Merkle_tree.

[UNISWAP-V2]  
Hayden Adams, Noah Zinsmeister, Dan Robinson, "Uniswap V2 Core", https://uniswap.org/whitepaper-v2.pdf.

---

## Appendix A: Formal Grammar for Symbol Encoding

ABNF grammar for SODS symbols (per RFC 5234):

```abnf
symbol           = opcode [ context ] [ offset ]
opcode           = 1*ALPHA / 1*DIGIT / (ALPHA *(ALPHA / DIGIT))
context          = "@" 1*(ALPHA / DIGIT)
offset           = "#d" 1*DIGIT

ALPHA            = %x41-5A / %x61-7A   ; A-Z / a-z
DIGIT            = %x30-39              ; 0-9
```

Examples:
- `Tf` matches `opcode = "Tf"`
- `LP+@U2` matches `opcode = "LP+", context = "@U2"`
- `Sw#d5` matches `opcode = "Sw", offset = "#d5"`

---

## Appendix B: Test Vectors

### B.1. BMT Construction

**Input Block**: Ethereum block with 3 logs

```json
{
  "block_number": 20000001,
  "block_hash": "0xabc...def",
  "logs": [
    {
      "address": "0xToken",
      "topics": ["0xddf252ad..."],  // ERC20 Transfer
      "data": "0x0000...0064"
    },
    {
      "address": "0xUniswapV2Pair",
      "topics": ["0xd78ad95f..."],  // Uniswap Swap
      "data": "0x0000...1234"
    },
    {
      "address": "0xToken",
      "topics": ["0xddf252ad..."],  // Another Transfer
      "data": "0x0000...0100"
    }
  ]
}
```

**Expected Encoding**:
```
Symbols (sorted):
  1. Sw (index 1) -> leaf_1 = SHA256("Sw" + metadata)
  2. Tf (index 0) -> leaf_2 = SHA256("Tf" + metadata)
  3. Tf (index 2) -> leaf_3 = SHA256("Tf" + metadata)

BMT Root Calculation:
  node_1 = SHA256(leaf_2 + leaf_3)  // Tf + Tf
  BMT_root = SHA256(leaf_1 + node_1)
```

**Expected Output**:
```
BMT_root: 0x3a7f9c2e1b5d8a6c4f2e9d1a8b3c6f5e2d4a7c9e1b5f8d3a6c9e2f5a8b1c4e
```

(Note: actual hash depends on metadata encoding; this is illustrative)

### B.2. Social Consensus Test Vector

Use this vector to test the voting/consensus algorithm logic.

**Input**: Peer responses for block `20000001`

```json
{
  "block": 20000001,
  "scenario": "Consensus with 2 agreeing, 1 Byzantine",
  "peers": [
    { "id": "A", "root": "0xabc...", "sig": "0x123..." },
    { "id": "B", "root": "0xabc...", "sig": "0x456..." },
    { "id": "C", "root": "0xdef...", "sig": "0x789..." }
  ],
  "expected": {
    "consensus_root": "0xabc...",
    "num_agreeing": 2,
    "confidence": 0.667,
    "byzantine_detected": true
  },
  "explanation": "Byzantine tolerance (f <= n/3) satisfied: 1 <= 3/3 ✓"
}
```

---

## Appendix C: P2P Message Serialization (JSON)

Messages are transmitted as JSON objects over libp2p streams. Alternative binary serialization (Protobuf, CBOR) may be defined in future extensions.

**Example BMT_CLAIM Message**:
```json
{
  "type": "BMT_CLAIM",
  "version": 1,
  "block_number": 20000001,
  "block_hash": "0xabc...def",
  "bmt_root": "0x3a7f9c2e1b5d8a6c4f2e9d1a8b3c6f5e2d4a7c9e1b5f8d3a6c9e2f5a8b1c4e",
  "timestamp": 1704635012,
  "signer": "0xNode1Address123...",
  "signature": "0x8f7a6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9f8e7d6c5b4a3f2e1d0c9b8a",
  "num_symbols": 3
}
```

---

## Appendix D: BMT Structure Diagram

```text
Block N (3 logs)
  ├─ Log 0: Tf
  ├─ Log 1: Sw
  └─ Log 2: Tf

Encoded & Sorted:
  [Tf₀, Tf₂, Sw₁]  (by blockindex, logindex, symbol)

Merkle Tree:
        BMT_root
        /       \
     node_1     leaf_Sw
    /      \
leaf_Tf  leaf_Tf
```

---

## Contributors

Feedback and suggestions for improvement are welcome via:
- GitHub Issues: https://github.com/logiccrafter/sods-protocol/issues
- Email: logiccrafterdz@gmail.com
- Community: Ethereum Research (ethereum-magicians.org), r/ethdev (Reddit)

---

## Author's Address

LogicCrafter  
Chlef, Algeria  
Email: logiccrafterdz@gmail.com  
GitHub: https://github.com/logiccrafterdz

---

## Acknowledgements

This specification was developed with inspiration from:
- RFC 791 (Internet Protocol) and RFC 7322 (RFC Style Guide) for structural and editorial guidance
- Ethereum Improvement Proposals (EIPs) and Bitcoin Improvement Proposals (BIPs) for decentralized standards processes
- Merkle tree research (Merkle, 1989) and peer-to-peer systems literature (Maymounkov & Mazières, 2002)

Special thanks to the Ethereum community for foundational work on decentralized systems and zero-trust verification principles.

---

**End of RFC Draft**
