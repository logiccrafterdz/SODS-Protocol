# SODS as an ERC-8004 Trustless Agent

SODS (Symbolic On-Demand Verification over Decentralized Summaries) is fully compliant with the [ERC-8004](https://eips.ethereum.org/EIPS/eip-8004) standard. It acts as a **Trustless Verification Agent** that provides behavioral validation as a service.

## Integration Overview

### 1. Discovery
You can discover the SODS agent via its registration file.
- **Registration Type**: `https://eips.ethereum.org/EIPS/eip-8004#registration-v1`
- **Identifier**: `eip155:11155111:0x...:22`

### 2. Behavioral Validation Service
SODS exposes a REST API for verifying causal behavioral proofs.

#### Endpoint: `POST /causal/verify`
Accepts a `CausalBehavioralProof` and returns a boolean result.

```json
{
  "pattern": { ... },
  "matched_events": [ ... ],
  "event_proofs": [ ... ],
  "agent_root": "0x..."
}
```

### 3. Reputation Feedback
Clients can submit feedback to the SODS Reputation Registry using the following tags:
- `behavioral_proof_accuracy`: Accuracy of the verification (0-100).
- `causal_verification_speed`: Response time in milliseconds.
- `agent_reliability`: Uptime and consistency (0-100).

---

## Developer Guide

### Validating a Claim
If your dApp needs to verify that an agent performed a specific sequence of actions (e.g., "executed 10 tasks successfully"), you can request a proof from SODS.

1.  **Request Proof**: `GET /causal/proof/{agent_id}`
2.  **Submit to Registry**: Include the proof hash in your on-chain validation request.
3.  **SODS Verification**: SODS will automatically pick up the request and submit a `validationResponse` (0 or 100) to the registry.

### CLI Usage
```bash
# Register your own agent identity
sods agent register --name "My Bot" --description "..." --endpoint "http://my-api.com"

# Start the SODS verification service
sods agent serve --port 8080
```
