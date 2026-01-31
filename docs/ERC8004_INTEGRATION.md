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
