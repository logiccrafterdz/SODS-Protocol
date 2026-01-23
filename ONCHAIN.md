# On-Chain Behavioral Proofs Verification

SODS enables DeFi protocols to natively react to on-chain behaviors (rug pulls, MEV, etc.) using compact, trustless proofs.

## Integration Guide

### 1. Include `SODSVerifier.sol`
Add [SODSVerifier.sol](file:///c:/Users/Hp/Desktop/SODS-Protocol/contracts/SODSVerifier.sol) to your project. This library is stateless and can be used directly or deployed as a shared library.

### 2. Verify Behavior in Your Contract
Use the library to validate behavioral sequences before executing critical logic.

```solidity
import "./SODSVerifier.sol";

contract BehavioralGuard {
    function ensureSafeState(
        uint256 blockNumber,
        uint256 chainId,
        string[] calldata symbols,
        uint32[] calldata logIndices,
        bytes32[] calldata leafHashes,
        bytes32[] calldata merklePath,
        bytes32 bmtRoot
    ) external {
        bool verified = SODSVerifier.verifyBehavior(
            blockNumber,
            chainId,
            symbols,
            logIndices,
            leafHashes,
            merklePath,
            bmtRoot
        );
        require(verified, "Behavior not verified");
        
        // React to verified behavior (e.g. pause trading if 'LP-' detected)
    }
}
```

### 3. Generate Proof via CLI
Use SODS CLI to generate the calldata needed for your contract:

```bash
sods export-proof --pattern "LP-" --block 123456 --chain base --format calldata
```

## Security & Costs

- **Gas Cost**: ~50,000 - 150,000 gas depending on proof depth and beacon root lookup.
- **Trust Model**: Verification relies on the provided `bmtRoot`. In **Trustless Mode**, this root is anchored to Ethereum's consensus via EIP-4788.

## Trustless On-Chain Verification (EIP-4788)

SODS now uses Ethereum's beacon root precompile to anchor BMT roots directly to the consensus layer, eliminating oracle dependency for behavioral verification.

### 1. Generate Anchored Proof
Use the `--anchored` flag to include the beacon root and block timestamp:
```bash
sods export-proof --pattern "LP-" --block 20000000 --chain ethereum --anchored
```

### 2. Verify with Anchor
The `SODSVerifier.verifyBehavior` function automatically validates the proof against the on-chain beacon root if provided.

```solidity
bool valid = SODSVerifier.verifyBehavior(
    blockNumber,
    chainId,
    symbols,
    indices,
    leafHashes,
    merklePath,
    bmtRoot,
    beaconRoot, // Fetched via CLI --anchored
    timestamp   // Block timestamp
);
```

> [!TIP]
> This requires a post-Dencun block on a network that supports the EIP-4788 precompile (0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02).
