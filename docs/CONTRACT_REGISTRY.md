# Contract Registry Schema

The SODS Contract Registry (`~/.sods/contract_registry.json`) is a local database that maps contract addresses to their deployers and deployment blocks. This enables faster verify operations by avoiding expensive on-chain deployer lookups.

## Schema Validation

Starting from v7.1, the contract registry is validated against a strict JSON schema on every load to prevent corruption and ensure consistency.

```json
{
  "version": "2.0",
  "contracts": {
    "0xContractAddress": {
      "deployer": "0xDeployerAddress",
      "block": 12345678,
      "name": "UniswapV2Router"
    }
  },
  "last_updated": 1675204800
}
```

### Constraints:
- **Contract Address**: Must be a valid Ethereum address (0x + 40 hex chars).
- **Deployer**: Must be a valid Ethereum address.
- **Block**: Must be a non-negative integer.
- **Version**: Patterned as `^\d+\.\d+$`.

## Version Migration

SODS automatically handles migration between registry versions to ensure your local data is always compatible with the latest engine:

- **v1.0 → v2.0**: 
  - Converts array-based entries `[addr, block]` into structured objects.
  - Adds the `version` field.
  - Ensures a `name` field exists for every entry (defaults to "Migrated").

Backward compatibility is maintained for at least 2 major versions.

## Error Handling

If the registry file fails validation (e.g., due to manual editing mistakes), SODS will display clear error messages:

```text
Contract registry validation failed:
  - contracts.0xInvalidAddr: Additional properties are not allowed ('0xInvalidAddr' was unexpected)
  - contracts.0x123...: missing required property "deployer"
```

In case of a non-recoverable schema mismatch, you may need to back up and recreate your `contract_registry.json`.
