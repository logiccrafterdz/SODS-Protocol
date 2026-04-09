// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/// @title SODSVerifier
/// @notice On-chain verification of SODS behavioral proofs.
/// @dev v0.2.0-beta: Beacon anchoring is best-effort.
/// On networks without EIP-4788, verification proceeds without consensus-layer binding.
/// This is explicitly documented as a known limitation.
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
    /// @dev Returns a tuple (proofValid, beaconAnchored) instead of a single bool.
    ///      - proofValid: true if the Merkle proof and leaf hashes are cryptographically valid.
    ///      - beaconAnchored: true if the beacon root was verified via EIP-4788.
    ///      Callers who require full security MUST check both values.
    ///      Callers on networks without EIP-4788 may accept proofValid alone,
    ///      understanding that the proof is not consensus-anchored.
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
    /// @return proofValid True if the Merkle proof is cryptographically valid.
    /// @return beaconAnchored True if the proof is anchored to the consensus layer via EIP-4788.
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
    ) external view returns (bool proofValid, bool beaconAnchored) {
        proofValid = false;
        beaconAnchored = false;

        // 1. Verify Commitment Signature (if provided)
        if (signature.length == 65 && trustedSigner != address(0)) {
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
            if (recovered != trustedSigner) return (false, false);
        }

        // 2. Anchor to Beacon Chain (EIP-4788) — best-effort
        if (beaconRoot != bytes32(0)) {
            try IBeaconRoots(BEACON_ROOTS_ADDRESS).getBeaconRoot(uint64(timestamp)) returns (bytes32 trustedRoot) {
                if (trustedRoot == beaconRoot) {
                    beaconAnchored = true;
                }
                // If trustedRoot != beaconRoot, beaconAnchored stays false
            } catch {
                // EIP-4788 not supported or contract not deployed
                // beaconAnchored stays false — caller decides how to handle
            }
        }

        // 3. Validate input arrays
        if (symbols.length == 0 || symbols.length != logIndices.length || symbols.length != leafHashes.length) {
            return (false, beaconAnchored);
        }

        // 4. Verify that leaf hashes match symbols and indices
        // Formula: keccak256(abi.encodePacked(symbol, logIndex))
        for (uint256 i = 0; i < symbols.length; i++) {
            if (keccak256(abi.encodePacked(symbols[i], logIndices[i])) != leafHashes[i]) {
                return (false, beaconAnchored);
            }
        }

        // 5. Verify Merkle Path
        if (merklePath.length != isLeftPath.length) {
            return (false, beaconAnchored);
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

        proofValid = (computedHash == bmtRoot);
        return (proofValid, beaconAnchored);
    }
}
