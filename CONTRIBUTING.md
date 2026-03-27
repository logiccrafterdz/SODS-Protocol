# Contributing to SODS Protocol

First off, thank you for considering contributing to the SODS Protocol!

We welcome pull requests from everyone. By participating in this project, you agree to abide by our codebase standards and review timelines.

## Getting Started

1. **Fork** the repository on GitHub.
2. **Clone** your fork locally.
3. Establish your development environment. You will need:
   - Rust (`cargo`, `rustc` > 1.70)
   - Foundry (`forge` for smart contract testing)

## Development Workflow

1. Always branch off from `main`.
2. Do your best to write descriptive commit messages that follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification.
   - Example: `feat(verifier): Implement Causal Merkle Tree optimizations`
   - Example: `fix(core): Resolve panic in pattern parsing quantifiers`
3. If you introduce a new feature, make sure to add it to the test suite (`sods-core/tests`, `sods-verifier/tests`).
4. Ensure all tests and linters pass:
   ```bash
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   ```
5. Push to your fork and submit a Pull Request.
6. A maintainer will review your code.

## Rust Code Standards
- Keep safety first: we use `unsafe` sparingly. If your PR uses `unsafe` blocks, it will undergo intense scrutiny. Please write extensive inline comments defending its usage.
- Error Handling: Do not use `unwrap()` or `expect()` outside of tests. Always bubble errors using `?` or `match`.

## Security Fixes
If you are contributing a fix for a security issue, **DO NOT PR DIRECTLY TO MAIN**.
Please review our [`SECURITY.md`](SECURITY.md) and disclose it via email first. We will invite you to a private vulnerability fork to build the patch securely.
