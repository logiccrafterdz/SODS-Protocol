# sods-cli

npm wrapper for SODS Protocol CLI.

Requires Docker to be installed.

This package provides a seamless way to run the SODS CLI using `npx` without needing to install the Rust toolchain manually.

## Installation

You don't need to install this package. Simply use `npx`:

```bash
npx sods-cli verify "Sandwich" --block 20000000 --chain ethereum
```

## Requirements

- **Docker**: Must be installed and running on your system.
- **Node.js**: Version 12 or higher.

## License

MIT
