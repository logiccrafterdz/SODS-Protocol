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

- **Gas Cost**: ~50,000 - 100,000 gas depending on proof depth.
- **Trust Model**: Verification relies on the provided `bmtRoot`. In production, this root should be sourced from a trusted commitment (e.g. a storage proof or a signed oracle update).

## Trustless Mode (Deep Verification)

SODS now cryptographically verifies that logs belong to the claimed block by recomputing the receipt trie root locally and comparing it against the block header's `receiptsRoot`. This ensures the RPC provider cannot omit or fabricate logs.

```bash
sods verify --mode trustless "Tf" --block 10002322 --chain sepolia
```
