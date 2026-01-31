# SODS Real-World Use Case: MEV Protection

This demo showcases how **SODS (ERC-8004 Agent)** protects DeFi protocols by providing trustless behavioral verification for AI trading agents.

## The Problem: AI Agent Transparency
In decentralized finance, AI agents are increasingly used for high-frequency trading and yield optimization. However, protocol owners face a dilemma:
- **Opacity**: How do we know the agent is actually executing the strategy as claimed?
- **MEV Exploitation**: How can we prove if an agent's "bad performance" was due to market conditions or if it was deliberately front-run or "sandwiched" due to poor execution logic?

## The Solution: SODS Behavioral Verification
SODS allows agents to record their internal decision-making events and results as a **Causal History**. This history is hashed into a **Causal Merkle Tree** and anchored on-chain.

Using **ERC-8004**, a DeFi protocol can set up an **Escrow contract** that only releases payments to the AI agent if SODS verifies a specific pattern of behavior (e.g., "10 consecutive profitable trades without sandwich loss").

---

## Running the Demo

### Prerequisites
- Rust and Cargo installed.
- PowerShell or Bash shell.

### Step 1: Start the SODS Agent
The SODS agent acts as the trustless verifier.
```bash
sods agent serve --port 8080
```

### Step 2: Run the DeFi Simulator
The simulator acts as the DeFi Protocol and the AI Agent. It records trades, generates a proof, and asks SODS to verify it.
```bash
cargo run -p defi-protocol
```

### Step 3: Observe the Workflow
1. **Recording**: AI Agent records 10 trades with "profit" status.
2. **Proof Generation**: Agent generates a cryptographic proof for these 10 events.
3. **Verification**: SODS receives the proof, validates the Merkle paths and the behavioral pattern, and returns `true`.
4. **Escrow Trigger**: The DeFi protocol receives the `valid: true` response and would typically trigger the `release()` function on the `Escrow` contract.

---

## Economic Impat
By using SODS, DeFi protocols can:
1. **Reduce Counterparty Risk**: Pay agents based on verified performance, not just reported numbers.
2. **Audit MEV Exposure**: Verify that execution patterns matches "healthy" behavior.
3. **Automate Trust**: Replace manual audits with cryptographic proofs.
