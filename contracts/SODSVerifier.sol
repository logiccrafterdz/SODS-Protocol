// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title SODSVerifier
/// @notice Library for verifying behavioral proofs on-chain.
library SODSVerifier {
    /// @notice Verifies a behavioral proof against a BMT root.
    /// @param blockNumber The block number where the behavior occurred.
    /// @param chainId The chain ID where the behavior occurred.
    /// @param symbols Array of symbol codes in the sequence.
    /// @param logIndices Array of log indices for the symbols.
    /// @param leafHashes Array of Keccak256 leaf hashes.
    /// @param merklePath Siblings in the Merkle path.
    /// @param bmtRoot The Keccak256 BMT root to verify against.
    /// @return bool True if the proof is valid.
    function verifyBehavior(
        uint256 blockNumber,
        uint256 chainId,
        string[] calldata symbols,
        uint32[] calldata logIndices,
        bytes32[] calldata leafHashes,
        bytes32[] calldata merklePath,
        bytes32 bmtRoot
    ) external pure returns (bool) {
        if (symbols.length == 0 || symbols.length != logIndices.length || symbols.length != leafHashes.length) {
            return false;
        }

        // 1. Verify that leaf hashes match symbols and indices
        // Requirement: keccak256(abi.encodePacked(symbol, logIndex))
        for (uint256 i = 0; i < symbols.length; i++) {
            if (keccak256(abi.encodePacked(symbols[i], logIndices[i])) != leafHashes[i]) {
                return false;
            }
        }

        // 2. Verify Merkle Path (inclusion of the first leaf)
        // For simplicity, we verify inclusion of the sequence starting from the first leaf's position.
        // In a true BMT, the 'matched' leaves would be part of a larger tree.
        // This verifier assumes the provided merklePath is for leafHashes[0].
        
        bytes32 computedHash = leafHashes[0];
        
        // Note: The off-chain generator provides siblings. 
        // We need to know if the sibling is left or right. 
        // In BMT, we can derive this from the leaf index if the tree is full.
        // However, for this PoC/Implementation, we expect the path to be ordered 
        // and we iterate upwards. A more robust implementation would include directions.
        
        // Standard Merkle verification loop (simplified: assuming siblings are correctly ordered)
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
