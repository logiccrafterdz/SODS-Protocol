# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2026-03-27

### Added
- **`SECURITY.md`** file for standardizing vulnerability disclosure and reporting protocols.
- **Foundry framework** in the `contracts/` directory enabling complete smart-contract behavioral test coverage.
- **Integration Tests** natively supporting `SEPOLIA_RPC_URL` testing within `sods-verifier`.
- **E2E CLI Pipeline test** inside `sods-cli` for guaranteeing output and argument robustness via `assert_cmd`.

### Changed
- **LruCache integration**: `sods-verifier` now uses standard LRU capacity controls inside RPC memory caching, decisively solving the previous unbounded `HashMap` memory leak issue.
- **Pattern Execution Context**: The execution state parser within `sods-core` has been audited with `proptest` ensuring maximum resilience against malicious DSL inputs (ReDoS protection).

### Fixed
- **Cryptographic Malleability**: The `Symbol` encoding within `sods-core` was patched to append the spatial data (`log_index`), eliminating a critical position spoofing vector in the root derivation.
- **Smart Contract Compilation**: Removed prohibited `emit` logic inside the `verifyBehavior` `view` function inside `contracts/SODSVerifier.sol`.
- **ZK Test Compilations**: Added missing registry arguments enforcing flawless `cargo check` and `sods-zk` functionality.

### Removed
- Removed misrepresentative "Production Ready" guarantees from `README.md` adjusting state explicitly to Alpha (Research).

## [1.0.0] - Initial Alpha Release
- Baseline SODS protocol establishing Behavioral Merkle Trees (BMT).
- Trustless header anchoring bridging L2 execution states to L1.
