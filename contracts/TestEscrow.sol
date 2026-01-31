// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title TestEscrow
 * @dev Simple escrow contract that releases payment based on ERC-8004 validation results.
 */
interface IERC8004ValidationRegistry {
    function getValidationResult(bytes32 requestId) external view returns (uint32 score, string memory metadata);
}

contract TestEscrow {
    address public protocol;
    address public agent;
    uint256 public amount;
    address public validationRegistry;
    bytes32 public validationRequestId;
    bool public released;

    event PaymentReleased(address to, uint256 amount);

    constructor(address _agent, address _validationRegistry) payable {
        protocol = msg.sender;
        agent = _agent;
        amount = msg.value;
        validationRegistry = _validationRegistry;
    }

    /**
     * @dev Releases payment if the validation score is 100.
     */
    function release(bytes32 requestId) external {
        require(!released, "Already released");
        
        (uint32 score, ) = IERC8004ValidationRegistry(validationRegistry).getValidationResult(requestId);
        
        require(score == 100, "Validation failed: Score must be 100");

        released = true;
        payable(agent).transfer(amount);
        
        emit PaymentReleased(agent, amount);
    }

    receive() external payable {}
}
