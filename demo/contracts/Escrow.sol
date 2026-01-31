// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

/**
 * @title Escrow
 * @dev Simple escrow contract that releases payment based on ERC-8004 validation results.
 */
interface IERC8004ValidationRegistry {
    function getValidationResult(bytes32 requestId) external view returns (uint32 score, string memory metadata);
}

contract Escrow {
    address public protocol;
    address public agent;
    uint256 public amount;
    bytes32 public validationRequestId;
    address public registry;
    bool public released;

    event PaymentReleased(address to, uint256 amount);
    event ValidationRequested(bytes32 requestId);

    constructor(address _agent, uint256 _amount, address _registry) {
        protocol = msg.sender;
        agent = _agent;
        amount = _amount;
        registry = _registry;
    }

    /**
     * @dev Sets the request ID from the ERC-8004 Validation Registry.
     */
    function setValidationRequest(bytes32 _requestId) external {
        require(msg.sender == protocol, "Only protocol can set request");
        validationRequestId = _requestId;
        emit ValidationRequested(_requestId);
    }

    /**
     * @dev Releases payment if the validation score is 100.
     */
    function release() external {
        require(!released, "Already released");
        
        (uint32 score, ) = IERC8004ValidationRegistry(registry).getValidationResult(validationRequestId);
        
        require(score == 100, "Validation failed: Agent behavior not verified");

        released = true;
        payable(agent).transfer(amount);
        
        emit PaymentReleased(agent, amount);
    }

    /**
     * @dev Allows the protocol to refund if needed (simplified for demo).
     */
    function refund() external {
        require(msg.sender == protocol, "Only protocol can refund");
        require(!released, "Already released");
        
        payable(protocol).transfer(address(this).balance);
    }

    receive() external payable {}
}
