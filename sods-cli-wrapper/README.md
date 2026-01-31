# sods-cli

Trustless behavioral verification for blockchain data.

This package provides a seamless way to run the SODS CLI using `npx` without needing to install the Rust toolchain manually. It wraps the official SODS Docker image for cross-platform reliability.

## Quick Start

```bash
npx sods-cli verify "Sandwich" --block 20000000 --chain ethereum
```

## Platform Support

| Platform | Node.js | Docker | npm Wrapper |
|----------|---------|--------|-------------|
| Linux    | ✅ 18+  | ✅ 20.10+ | ✅         |
| macOS    | ✅ 18+  | ✅ 20.10+ | ✅         |
| Windows  | ✅ 18+  | ✅ 20.10+ | ✅ (WSL2)  |

> [!NOTE]
> Windows requires Docker Desktop with the WSL2 backend enabled.

## Requirements

- **Docker**: Must be installed and running. [Get Docker](https://docs.docker.com/get-docker/)
- **Node.js**: Version 18.0 or higher.

## License

MIT
