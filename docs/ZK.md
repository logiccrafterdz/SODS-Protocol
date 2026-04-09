# Zero-Knowledge Behavioral Proofs (ZKBP)

SODS leverages RISC Zero's zkVM to generate **Zero-Knowledge Behavioral Proofs**. This allows a user to prove that a specific behavioral pattern occurred in a block without revealing sensitive metadata like specific addresses, transaction amounts, or the full sequence of other symbols in that block.

## How it Works

1.  **Symbol Extraction**: SODS fetches logs for a requested block and parses them into behavioral symbols.
2.  **zkVM Execution**: The symbols and the target pattern are passed into the RISC Zero zkVM.
3.  **Pattern Matching**: The same `sods-core` pattern matching logic runs inside the zkVM.
4.  **STARK Proof**: The zkVM generates a STARK receipt that serves as a proof of computation.
5.  **Public Journal**: The only public output is a single boolean: `valid` (true/false).

## Usage

### Generate a ZK Proof
To generate a ZK proof for a pattern in a specific block:

```bash
sods zk-prove --pattern "LP+ -> Sw -> LP-" --block 10002322 --chain base
```

This will output:
- `proof.bin`: The RISC Zero STARK receipt (~100KB-200KB).
- `public.json`: A summary including the validity result.

### Verification on Ethereum
You can verify the generated `proof.bin` on-chain using the RISC Zero Ethereum Verifier.

#### Solidity Integration
```solidity
import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";

contract SODSZKVerifier {
    IRiscZeroVerifier public verifier;
    bytes32 public constant IMAGE_ID = 0x...; // The SODS ZK Guest Image ID

    constructor(address _verifier) {
        verifier = IRiscZeroVerifier(_verifier);
    }

    function verifyBehavior(bytes calldata proof, bool expectedResult) external view {
        verifier.verify(proof, IMAGE_ID, keccak256(abi.encode(expectedResult)));
    }
}
```

## Security & Privacy
- **Metadata Blindness**: The proof does not contain individual symbol details or account addresses.
- **Integrity**: Because the proof is generated inside a zkVM, it is mathematically impossible to forge a "valid" result for a pattern that did not occur.
- **Fail-Open/Fail-Closed**: The public output is binary. If a pattern doesn't match, the proof will validly state `false`.
