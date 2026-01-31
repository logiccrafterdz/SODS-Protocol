# Contract Registry Schema

## Contract Registry Schema

The contract registry (`~/.sods/contract_registry.json`) follows this schema:

```json
{
  "version": "1.0",
  "contracts": {
    "0xContractAddress": {
      "deployer": "0xDeployerAddress",
      "block": 12345678,
      "name": "UniswapV3Pool"
    }
  }
}
```

### Migration
- Version 1.0 → 2.0: Automatic migration on first load
- Backward compatibility maintained for 2 versions
