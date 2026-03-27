# Security Policy

## Supported Versions

Currently, the SODS Protocol is in **Alpha (Research Prototype)**. We only provide security updates for the latest code on the `main` branch.

| Version | Supported          |
| ------- | ------------------ |
| `main`  | :white_check_mark: |
| `< 1.0` | :x:                |

## Reporting a Vulnerability

We take the security of our protocol and our users' funds extremely seriously. If you believe you've found a security vulnerability in the SODS Protocol (Core, Verifier, CLI, or Smart Contracts), please report it to us confidentially before disclosing it publicly.

**DO NOT** create a public GitHub issue for security vulnerabilities.

### How to Report
1. Email your findings to: `logiccrafterdz@gmail.com`
2. Please include:
   - A description of the vulnerability.
   - The files/components affected (e.g., `sods-core/pattern.rs`, `contracts/SODSVerifier.sol`).
   - A proof-of-concept (PoC) or instructions to reproduce the issue.
   - Any suggested mitigations.

### Scope
**In Scope:**
- Cryptographic bypasses in `leaf_hash` or Behavioral Merkle Tree construction.
- Malicious RPC extraction flaws (e.g., accepting unverified logs in Trustless mode).
- Denial of Service (DoS) / Cache stampedes in the continuous verifier (daemon mode).
- Smart contract logic errors in `contracts/*.sol`.

**Out of Scope:**
- Issues relying on social engineering or physical access.
- Bugs in third-party RPC endpoints (e.g., Infura, Alchemy downtime).
- Known issues explicitly documented in the `README.md`.

### Response Timeline
- We will acknowledge receipt of your vulnerability report within **48 hours**.
- We aim to triage and provide a preliminary assessment within **7 days**.
- If a fix is needed, we will coordinate the disclosure timeline with you.

Thank you for helping keep the SODS Protocol safe!
