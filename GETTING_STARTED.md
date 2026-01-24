# Getting Started with SODS

## Installation

1. Ensure you have Rust installed.
2. Clone the repository:
   ```bash
   git clone https://github.com/logiccrafterdz/SODS-Protocol.git
   cd SODS-Protocol
   ```
3. Build the CLI:
   ```bash
   cargo build --release -p sods-cli
   ```
4. Run:
   ```bash
   target/release/sods --help
   ```

## Basic Usage

### Verify a Symbol
Check if a specific event occurred in a block:

```bash
sods verify Tf --block 10002322 --chain sepolia
```

### Detect Behavioral Trends
Identify emerging patterns (e.g., transfers, swaps, liquidity events) across recent blocks:

```bash
# Scan last 10 blocks on Base for "LP+ -> Sw" pattern
sods trend --pattern "LP+ -> Sw" --chain base --window 10

# Scan last 50 blocks on Arbitrum for high-frequency transfers
sods trend --pattern "Tf{5,}" --chain arbitrum --window 50
```

### Autonomous Monitoring (with Adaptive RPC)
Continuously monitor the chain for emerging patterns. Use `--auto-adapt` to automatically throttle requests if the RPC provider limits you:

```bash
# Monitor with self-healing RPC logic (auto-throttles on 429 errors)
sods monitor --pattern "Sw{3,}" --chain base --interval 10s --auto-adapt
```

### Dynamic Symbol Loading (New!)
Load custom behavioral symbols from external JSON plugins (e.g., from IPFS or GitHub) without updating the binary.

```bash
# Load Uniswap V3 Swap symbol from a plugin
sods symbols load https://raw.githubusercontent.com/sods/plugins/main/uniswap-v3.json

# Now verify using the new symbol "SwV3"
sods verify "SwV3" --block 123456 --chain ethereum
```

### Predictive Behavioral Shadowing (Proactive)
Enable proactive shadowing to detect pattern initiations (e.g., `LP+`) and receive alerts if the expected sequence (e.g., `LP+ -> Sw`) deviates or times out before completion.

```bash
# Monitor for rug pulls with predictive alerts (WARN if LP+ happens but LP- is missing)
sods monitor --pattern "LP+ -> Sw -> LP-" --chain base --enable-shadows
```

### Real-Time Mempool Monitoring (New!)
Monitor **pending transactions** in real-time to detect threats before they are even mined:

```bash
# Monitor Base mempool for Sandwich attacks (Pending transactions)
sods monitor --pattern "Sandwich" --chain base --mode pending
```


## Privacy-Preserving Alerts (New!)

Webhook alerts use **pattern hashing** to prevent privacy leaks. Attackers observing your webhook traffic cannot learn which specific behaviors you are monitoring.

To correlate alerts with your rules, use the `hash-pattern` utility:

```bash
# Compute the privacy-safe hash for your pattern
sods hash-pattern "LP+ -> Sw{3,} -> LP-"
# Output: 0x8a3b7c...
```

When an alert is received, look for the `pattern_hash` field to identify which rule triggered.

### Run as a System Daemon (Linux/macOS)
Run SODS as a background service with desktop notifications and private webhook alerts.

```bash
# Start daemon with private webhook
sods daemon start --pattern "Tf{2,}" --chain base --webhook-url "https://ntfy.sh/my_alerts"
```

### 🛡️ Long-Running Stability (v2.1)
SODS is designed for 24/7 background operation. To prevent unbounded memory growth, especially when connected to high-frequency P2P threat feeds:
- **Auto-Expiration**: Rules automatically expire after **24 hours** by default.
- **Garbage Collection**: Every 5 minutes, SODS prunes expired rules from memory.
- **Custom Expiration**: Use `--expire-after` to tune the rule lifespan.
  ```bash
  # Keep rules for only 1 hour
  sods daemon start --expire-after 1h
  
  # Keep rules for 30 minutes
  sods daemon start --expire-after 30m
  ```

## Monitor Community Threat Feeds
Protect yourself by subscribing to public behavioral blocklists (e.g., known rug pull patterns).

```bash
# Start daemon with community threat feed
sods daemon start --threat-feed "https://raw.githubusercontent.com/sods/threats/main/base.json" --chain base --webhook-url "https://ntfy.sh/my_alerts"
```

### Discover Active Blocks
Find blocks with high activity for a specific symbol:

```bash
sods discover --symbol Sw --chain base --last 20
```

## Decentralized Threat Intelligence (P2P)

Join the global P2P network to receive real-time behavioral threat updates without centralized feeds.

```bash
# Start daemon with P2P enabled
sods daemon start --p2p-threat-network --chain base

# List active P2P rules received
sods threats list

# Trust a new researcher (add their public key)
sods threats add-key 02a1b2c3...
```

## Detect Advanced Threats

### MEV Detection (Sandwich Attacks)
Detect sandwich attacks where a victim's swap is bracketed by attacker transactions:

```bash
# Verify sandwich pattern in a specific block
sods verify "Sandwich" --block 20000000 --chain ethereum

# Monitor for real-time sandwich attacks on Base
sods monitor --pattern "Sandwich" --chain base --interval 5s
```

### NFT Activity Monitoring
Track NFT mints, listings, and purchases:

```bash
# Detect NFT mints on Base
sods trend --pattern "MintNFT" --chain base --window 50

# Monitor Seaport NFT purchases
sods verify "BuyNFT" --block 19500000 --chain ethereum

# Detect Blur listings
sods verify "ListNFT" --block 19500000 --chain ethereum
```

### Cross-Chain Bridge Monitoring
Monitor L1↔L2 bridge activity with future-proof event signature resolution:

```bash
# Detect bridge deposits on Optimism (Standard Signature)
sods verify "BridgeIn" --block 115000000 --chain optimism

# Monitor bridge withdrawals on Scroll (Dynamic Resolution)
sods trend --pattern "BridgeOut" --chain scroll --window 20
```

> [!NOTE]
> SODS uses dynamic event signature hashing (EVM standard) for bridge events, ensuring monitoring remains robust even if L2 contracts are upgraded or redeployed.

### MEV Frontrun/Backrun Detection
Detect frontrunning and backrunning patterns:

```bash
# Detect frontrun pattern (Transfer before Swap)
sods verify "Frontrun" --block 20000000 --chain ethereum

# Detect backrun pattern (Swap followed by Transfer)
sods verify "Backrun" --block 20000000 --chain ethereum
```

### Next-Gen Behavior Monitoring (v2.2)
Monitor emerging Web3 paradigms like Account Abstraction, Permit2, and CoW Swap:

```bash
# Detect ERC-4337 UserOperation executions (AAOp)
sods verify "AAOp" --block 20000000 --chain ethereum

# Monitor Permit2 gasless approvals on Base
sods trend --pattern "Permit2" --chain base --window 50

# Verify CoW Swap intent fulfillments
sods verify "CoWTrade" --block 20000000 --chain ethereum
```

### Context-Aware Patterns (New in v1.2)
Enhance your detection with contextual conditions like `from == deployer` and `value` comparisons.

```bash
# Monitor for LP removal by deployer (high confidence rug indicator)
sods monitor --pattern "LP+ where from == deployer -> Sw{2,} -> LP-" --chain base --interval 10s

# Verify high-value transfers in a block
sods verify "Tf where value > 50 ether" --block 20000000 --chain ethereum

# Detect large swaps on Base
sods verify "Sw where value > 10000 gwei" --block 9000000 --chain base
```
## Reliable Operation on L2s (v1.3)

SODS is hardened for unreliable L2 public RPCs (Scroll, Polygon zkEVM, Base, etc.). It automatically ensures uninterrupted operation through:

- **Multi-Endpoint Fallback**: Each supported chain is configured with multiple diverse RPC providers (e.g., official, PublicNode, 1RPC). If one fails, SODS silently fails over to the next.
- **L2-Aware Backoff**: Specialized exponential backoff profiles for L2s to respect stricter rate limits.
- **Pre-Flight Health Checks**: SODS validates RPC health before starting long-running `monitor` or `trend` sessions.

### Using Custom RPCs with Fallback
If you provide a custom RPC URL via `--rpc-url`, SODS will use that as the primary endpoint but still maintain the ability to fallback to public defaults if yours becomes unavailable.

## On-Chain Verification (New in v1.4)

Generate behavioral proofs that can be verified inside Ethereum smart contracts:

```bash
sods export-proof --pattern "Sandwich" --block 20000000 --chain ethereum --format calldata
```

**[Read the On-Chain Verification Guide](ONCHAIN.md)**

## Network Compatibility

SODS automatically adapts to the security capabilities of the underlying network.

| Network | Beacon Root Support | Security Level |
|---------|---------------------|----------------|
| Ethereum (post-Dencun) | ✅ Full (EIP-4788) | Highest |
| Ethereum (pre-Dencun) | ⚠️ Fallback | Medium |
| L2s (Arbitrum, Base, etc.) | ⚠️ Fallback | Medium |
| Testnets | ⚠️ Fallback | Medium |

> [!NOTE]
> Fallback mode skips beacon root verification but continues full behavioral Merkle proof analysis. This ensures functionality on all EVM chains while maintaining transparency about reduced block header anchoring guarantees.
