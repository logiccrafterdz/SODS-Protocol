# SODS as an ERC-8004 Trustless Agent

SODS is registered as a compliant ERC-8004 agent providing behavioral verification services.

## Agent Registration
- **Identifier**: `eip155:11155111:0x[registry]:[agentId]` (Sepolia)
- **Services**: REST API, A2A, MCP endpoints
- **Supported Trust**: reputation, zk-proofs, crypto-economic

## Integration Workflow
1. Request validation via Validation Registry
2. SODS verifies behavioral proof cryptographically
3. Validation result published on-chain (0-100 score)
4. Escrow contract releases payment based on result

## Example Usage
```bash
sods agent register --name "My Verifier" --endpoint "https://api.myapp.com"
```

## End-to-End Testing

SODS includes a comprehensive integration test suite for ERC-8004:
- **Agent Registration**: Verifies on-chain identity and metadata accessibility.
- **Validation Cycle**: Simulates the full request -> verify -> respond loop.
- **Reputation**: Validates the submission of quality metrics.
- **Escrow**: Confirms payment release based on validation scores.

To run tests on Sepolia:
```bash
export SEPOLIA_RPC_URL="your_rpc_url"
export TEST_PRIVATE_KEY="your_private_key"
cargo test --package sods-cli --test erc8004_integration
```
