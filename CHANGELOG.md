# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0-beta] - 2026-04-09

### BREAKING CHANGES
- **Keccak256 Default**: BMT now uses Keccak256 exclusively (was SHA-256). All prior roots are incompatible.
- **API Rename**: `build_incremental()` → `from_filtered()` on `BehavioralMerkleTree`.
- **API Removal**: `new_keccak()` and `leaf_hash_keccak()` removed — `new()` and `leaf_hash()` now use Keccak256.
- **Leaf Hash Formula**: Now `Keccak256(symbol_bytes || BigEndian_u32(log_index))` — matches `SODSVerifier.sol`.
- **License Change**: CC0-1.0 → MIT OR Apache-2.0 (Rust ecosystem standard).

### Added
- `test_leaf_hash_matches_solidity_abi_encode_packed` test proving Rust↔Solidity hash parity.
- Multi-OS CI matrix (Ubuntu, Windows, macOS) with `-D warnings` zero-tolerance policy.
- Cross-layer consistency test as a dedicated CI step.
- `LICENSE-MIT` and `LICENSE-APACHE` files.

### Fixed
- Hash algorithm mismatch between Rust core (SHA-256) and SODSVerifier.sol (Keccak256).
- README code example using wrong `BehavioralSymbol::new()` signature (3 args → 2 args).
- Unbounded `pattern_cache` HashMap in verifier (now LruCache with 500 entry limit).
- Typo in proof.rs: "Contrcat" → "Contract".
- SODS-SPEC §5.1 leaf hash formula now matches implementation.

### Changed
- Unified versioning across all 5 crates to `0.2.0-beta`.
- Removed "Vision 2126 Edition" / "Neural Overlay" branding from README.
- Updated project status from "Research Prototype" to "Pre-Production".

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
