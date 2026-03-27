// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../SODSVerifier.sol";

contract SODSVerifierTest is Test {
    SODSVerifier public verifier;

    function setUp() public {
        verifier = new SODSVerifier();
    }

    function test_VerifyBehavior_Success() public {
        string memory pattern = "Tf";
        bytes[] memory symbols = new bytes[](1);
        symbols[0] = abi.encodePacked("Tf", uint32(0)); // leaf_hash logic

        // Generate the leaf hash the same way SODSVerifier does:
        // leafHash = keccak256(symbols[0])
        bytes32 leafHash = keccak256(symbols[0]);
        
        // Let's assume a tree of size 1, so root == leafHash
        bytes32 trustedRoot = leafHash;
        
        // Empty proof because it's the root itself
        bytes32[] memory proof = new bytes32[](0);
        bool[] memory isLeftPath = new bool[](0);

        bool result = verifier.verifyBehavior(
            pattern,
            symbols,
            123456,
            trustedRoot,
            proof,
            isLeftPath
        );

        assertTrue(result, "Verification should succeed for valid single-node tree");
    }

    function test_VerifyBehavior_InvalidRoot() public {
        string memory pattern = "Tf";
        bytes[] memory symbols = new bytes[](1);
        symbols[0] = abi.encodePacked("Tf", uint32(0));

        bytes32 fakeRoot = keccak256(abi.encodePacked("FAKE"));
        bytes32[] memory proof = new bytes32[](0);
        bool[] memory isLeftPath = new bool[](0);

        // Verification must fail because the derived root won't match the fakeRoot
        vm.expectRevert("BMT Proof verification failed");
        verifier.verifyBehavior(
            pattern,
            symbols,
            123456,
            fakeRoot,
            proof,
            isLeftPath
        );
    }
}
