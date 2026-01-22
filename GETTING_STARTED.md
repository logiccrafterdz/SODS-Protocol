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


### Run as a System Daemon (Linux/macOS)
Run SODS as a background service with desktop notifications.

```bash
# Start daemon
sods daemon start --pattern "Tf{2,}" --chain base --autostart

# Check status
sods daemon status

# Stop daemon
sods daemon stop
```

## Forward Alerts to Your Phone / Discord / Telegram
Forward alerts to any service by providing a secure HTTPS webhook URL:

```bash
# Example: Send alerts to ntfy.sh (receive on phone)
sods daemon start --pattern "Tf{2,}" --chain base --webhook-url "https://ntfy.sh/my_sods_alerts"

# Example: Send alerts to Discord Webhook
sods daemon start --pattern "Sw" --chain base --webhook-url "https://discord.com/api/webhooks/YOUR_ID/YOUR_TOKEN"
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
