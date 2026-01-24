# On-Chain Behavioral Proofs

---

## Zero-RPC Verification

SODS now uses Ethereum storage proofs (Merkle-Patricia Trie proofs) to fetch and verify transaction receipts without trusting RPC logs. 

### How it works:
1. **Header Anchoring**: Fetch block header and verify its hash.
2. **Path Proof**: Request the MPT proof for a specific receipt index from a provider.
3. **Local MPT Verifier**: Reconstruct the path and verify the leaf (receipt) matches the `receiptsRoot` in the header.
4. **Log Extraction**: Extract behavioral symbols from the cryptographically-proven receipt.

**Verification Modes:**
- `storage-proof` (Zero-RPC): Proof-first, highest security.
- `trustless`: Bulk receipt verification.
- `rpc-only`: Legacy mode (not recommended).

---
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
Use SODS CLI to generate the calldata needed for your contract. SODS now uses the `ethabi` library (v2.0 ABI) for 100% Solidity compatibility.

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

## Signed Behavioral Commitments

For maximal security, SODS can sign a commitment that binds the BMT root to the block's `receiptsRoot`. This prevents any tampering with the BMT structure off-chain.

### 1. Generate Signed Proof
Provide a private key and the expected trusted signer address:
```bash
sods export-proof --pattern "LP-" --block 20000000 --chain ethereum \
  --anchored \
  --signing-key 0x... \
  --trusted-signer 0x...
```

### 2. Verify On-Chain
The contract will recover the signer from the signature and compare it against the `trustedSigner` address.

```solidity
SODSVerifier.verifyBehavior(
    blockNumber,
    chainId,
    symbols,
    indices,
    leafHashes,
    merklePath,
    bmtRoot,
    beaconRoot,
    timestamp,
    receiptsRoot,
    signature,
    trustedSigner
);
```

### 3. Security Notes
- **Replay Protection**: The commitment includes `chainId` and `blockNumber`.
- **Integrity**: The signature covers both the block identifiers and the roots, ensuring the BMT matches the specific block logs.

---

## v3 ABI: Explicit Merkle Path Ordering

**New in v3**: Proofs now include an `isLeftPath` boolean array to resolve ordering ambiguity between off-chain (Rust) and on-chain (Solidity) verification.

### Why This Change?

Previously, `SODSVerifier.sol` used hash-comparison (`computedHash < sibling`) to determine ordering. This did not match the index-based ordering in the Rust BMT implementation, causing valid proofs to be rejected on-chain.

### New Parameter

| Field | Type | Description |
|-------|------|-------------|
| `isLeftPath` | `bool[]` | Direction for each sibling: `true` = leaf is left child (sibling on right) |

### Updated Function Signature

```solidity
function verifyBehavior(
    uint256 blockNumber,
    uint256 chainId,
    string[] calldata symbols,
    uint32[] calldata logIndices,
    bytes32[] calldata leafHashes,
    bytes32[] calldata merklePath,
    bool[] calldata isLeftPath,  // NEW in v3
    bytes32 bmtRoot,
    bytes32 beaconRoot,
    uint256 timestamp,
    bytes32 receiptsRoot,
    bytes calldata signature,
    address trustedSigner
) external view returns (bool);
```

### Generating v3 Proofs

```bash
sods export-proof --pattern "Tf" --block 20000000 --chain ethereum --format calldata
```

The CLI automatically includes `isLeftPath` in the generated calldata.

> [!IMPORTANT]
> v3 proofs are **not backward compatible** with v2 contracts. Deploy the updated `SODSVerifier.sol` before submitting v3 proofs.
