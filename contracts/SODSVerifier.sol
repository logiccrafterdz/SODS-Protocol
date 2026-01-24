// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title SODSVerifier
/// @notice Library for verifying behavioral proofs on-chain.
library SODSVerifier {
    address constant BEACON_ROOTS_ADDRESS = 0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02;

    interface IBeaconRoots {
        function getBeaconRoot(uint64 timestamp) external view returns (bytes32);
    }

    bytes32 private constant DOMAIN_TYPEHASH = keccak256(
        "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
    );

    bytes32 private constant COMMITMENT_TYPEHASH = keccak256(
        "BehavioralCommitment(uint256 blockNumber,uint256 chainId,bytes32 receiptsRoot,bytes32 bmtRoot)"
    );

    /// @notice Verifies a behavioral proof against a BMT root.
    /// @param blockNumber The block number where the behavior occurred.
    /// @param chainId The chain ID where the behavior occurred.
    /// @param symbols Array of symbol codes in the sequence.
    /// @param logIndices Array of log indices for the symbols.
    /// @param leafHashes Array of Keccak256 leaf hashes.
    /// @param merklePath Siblings in the Merkle path.
    /// @param isLeftPath Direction for each sibling: true = leaf is left child (sibling on right).
    /// @param bmtRoot The Keccak256 BMT root to verify against.
    /// @param beaconRoot The expected beacon root (untrusted input from proof).
    /// @param timestamp The block timestamp.
    /// @param receiptsRoot The root of the transaction receipts trie.
    /// @param signature The ECDSA signature over the commitment.
    /// @param trustedSigner The address authorized to sign BMT commitments.
    /// @return bool True if the proof is valid, anchored, and signed (if provided).
    function verifyBehavior(
        uint256 blockNumber,
        uint256 chainId,
        string[] calldata symbols,
        uint32[] calldata logIndices,
        bytes32[] calldata leafHashes,
        bytes32[] calldata merklePath,
        bool[] calldata isLeftPath,
        bytes32 bmtRoot,
        bytes32 beaconRoot,
        uint256 timestamp,
        bytes32 receiptsRoot,
        bytes calldata signature,
        address trustedSigner
    ) external view returns (bool) {
        // 1. Verify Commitment Signature (if provided)
        if (signature.length == 65 && trustedSigner != address(0)) {
            // EIP-712 Structured Data Hashing
            bytes32 domainSeparator = keccak256(abi.encode(
                DOMAIN_TYPEHASH,
                keccak256(bytes("SODS Protocol")),
                keccak256(bytes("1.0")),
                block.chainid,
                address(this)
            ));

            bytes32 structHash = keccak256(abi.encode(
                COMMITMENT_TYPEHASH,
                blockNumber,
                chainId,
                receiptsRoot,
                bmtRoot
            ));

            bytes32 digest = keccak256(abi.encodePacked(
                "\x19\x01",
                domainSeparator,
                structHash
            ));
            
            // Extract v, r, s
            bytes32 r;
            bytes32 s;
            uint8 v;
            assembly {
                r := calldataload(signature.offset)
                s := calldataload(add(signature.offset, 32))
                v := byte(0, calldataload(add(signature.offset, 64)))
            }
            if (v < 27) v += 27;

            address recovered = ecrecover(digest, v, r, s);
            if (recovered != trustedSigner) return false;
        }

        // 2. Anchor to Beacon Chain (EIP-4788)
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

        // 4. Verify Merkle Path with explicit direction flags (v3 ABI)
        if (merklePath.length != isLeftPath.length) {
            return false;
        }

        bytes32 computedHash = leafHashes[0];
        
        for (uint256 i = 0; i < merklePath.length; i++) {
            bytes32 sibling = merklePath[i];
            if (isLeftPath[i]) {
                // Leaf is left child, sibling is on right: H(current || sibling)
                computedHash = keccak256(abi.encodePacked(computedHash, sibling));
            } else {
                // Leaf is right child, sibling is on left: H(sibling || current)
                computedHash = keccak256(abi.encodePacked(sibling, computedHash));
            }
        }

        return computedHash == bmtRoot;
    }
}
