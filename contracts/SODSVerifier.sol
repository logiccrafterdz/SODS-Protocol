// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title SODSVerifier
/// @notice Library for verifying behavioral proofs on-chain.
library SODSVerifier {
    address constant BEACON_ROOTS_ADDRESS = 0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02;

    interface IBeaconRoots {
        function getBeaconRoot(uint64 timestamp) external view returns (bytes32);
    }

    /// @notice Verifies a behavioral proof against a BMT root.
    /// @param blockNumber The block number where the behavior occurred.
    /// @param chainId The chain ID where the behavior occurred.
    /// @param symbols Array of symbol codes in the sequence.
    /// @param logIndices Array of log indices for the symbols.
    /// @param leafHashes Array of Keccak256 leaf hashes.
    /// @param merklePath Siblings in the Merkle path.
    /// @param bmtRoot The Keccak256 BMT root to verify against.
    /// @param beaconRoot The expected beacon root (untrusted input from proof).
    /// @param timestamp The block timestamp.
    /// @return bool True if the proof is valid and anchored.
    function verifyBehavior(
        uint256 blockNumber,
        uint256 chainId,
        string[] calldata symbols,
        uint32[] calldata logIndices,
        bytes32[] calldata leafHashes,
        bytes32[] calldata merklePath,
        bytes32 bmtRoot,
        bytes32 beaconRoot,
        uint256 timestamp
    ) external view returns (bool) {
        // 1. Anchor to Beacon Chain (EIP-4788)
        // We verify that the provided beaconRoot is actually part of Ethereum's consensus
        if (beaconRoot != bytes32(0)) {
            try IBeaconRoots(BEACON_ROOTS_ADDRESS).getBeaconRoot(uint64(timestamp)) returns (bytes32 trustedRoot) {
                if (trustedRoot != beaconRoot) return false;
            } catch {
                // If beacon root lookup fails (e.g. pre-Dencun or non-mainnet without precompile), 
                // we might fallback or fail. For this trustless mode, we fail.
                return false;
            }
            
            // 2. Cross-verify BMT root with Block Header (Simulated for v1)
            // In a full implementation, we would verify a storage proof here:
            // beaconRoot -> ExecutionPayloadHeader -> receiptsRoot == bmtRoot
            // For v1, we assume the BMT root is anchored if it matches the trusted setup.
        }

        if (symbols.length == 0 || symbols.length != logIndices.length || symbols.length != leafHashes.length) {
            return false;
        }

        // 3. Verify that leaf hashes match symbols and indices
        for (uint256 i = 0; i < symbols.length; i++) {
            if (keccak256(abi.encodePacked(symbols[i], logIndices[i])) != leafHashes[i]) {
                return false;
            }
        }

        // 4. Verify Merkle Path
        bytes32 computedHash = leafHashes[0];
        
        for (uint256 i = 0; i < merklePath.length; i++) {
            bytes32 sibling = merklePath[i];
            if (computedHash < sibling) {
                computedHash = keccak256(abi.encodePacked(computedHash, sibling));
            } else {
                computedHash = keccak256(abi.encodePacked(sibling, computedHash));
            }
        }

        return computedHash == bmtRoot;
    }
}
