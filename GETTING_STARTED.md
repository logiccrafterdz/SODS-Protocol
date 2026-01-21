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

### Autonomous Monitoring
Continuously monitor the chain for emerging patterns:

```bash
# Alert when 3 or more swaps occur in a sequence
sods monitor --pattern "Sw{3,}" --chain base --interval 30s
```

### Discover Active Blocks
Find blocks with high activity for a specific symbol:

```bash
sods discover --symbol Sw --chain base --last 20
```
