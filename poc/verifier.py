"""
SODS PoC v0.1 — Proof Verifier

CLI tool to verify that a behavioral symbol exists in a block
by validating its Merkle proof against the stored BMR.

Usage:
    python src/verifier.py --symbol LP+ --block 5000000 --local

Output:
    [+] Verified: LP+ exists in block 5000000
    Proof size: 287 bytes
    Verification time: < 20 ms
    Cost: $0.00
"""

import argparse
import sys
import time
from pathlib import Path

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent))

from config import BMR_FILE, PROOFS_DIR, TARGET_BLOCK
from merkle import MerkleTree, hash_symbol


def load_bmr() -> bytes:
    """
    Load the Behavioral Merkle Root from file.
    
    Returns:
        32-byte root hash
        
    Raises:
        FileNotFoundError: If bmr.bin doesn't exist
    """
    if not BMR_FILE.exists():
        raise FileNotFoundError(
            f"BMR file not found: {BMR_FILE}\n"
            "Run bmt_builder.py first to generate the BMR."
        )
    
    root = BMR_FILE.read_bytes()
    if len(root) != 32:
        raise ValueError(f"Invalid BMR: expected 32 bytes, got {len(root)}")
    
    return root


def load_proof(symbol: str, block_number: int) -> tuple:
    """
    Load proof file for a symbol.
    
    Args:
        symbol: The behavioral symbol (e.g., "LP+")
        block_number: The block number
        
    Returns:
        Tuple of (leaf_hash, leaf_index, proof_bytes)
        
    Raises:
        FileNotFoundError: If proof file doesn't exist
    """
    # Sanitize symbol for filename
    safe_symbol = symbol.replace("+", "_plus").replace("-", "_minus")
    proof_file = PROOFS_DIR / f"{safe_symbol}_{block_number}.proof"
    
    if not proof_file.exists():
        raise FileNotFoundError(
            f"Proof file not found: {proof_file}\n"
            f"Symbol '{symbol}' may not exist in block #{block_number}."
        )
    
    proof_data = proof_file.read_bytes()
    
    # Parse proof file format:
    # - 32 bytes: leaf hash
    # - 4 bytes: leaf index (big-endian)
    # - N bytes: proof data
    
    if len(proof_data) < 36:
        raise ValueError(f"Invalid proof file: too short ({len(proof_data)} bytes)")
    
    leaf_hash = proof_data[:32]
    leaf_index = int.from_bytes(proof_data[32:36], 'big')
    proof_bytes = proof_data[36:]
    
    return leaf_hash, leaf_index, proof_bytes, len(proof_data)


def verify_proof(symbol: str, leaf_hash: bytes, proof_bytes: bytes, expected_root: bytes) -> bool:
    """
    Verify a Merkle proof.
    
    Args:
        symbol: The symbol being verified
        leaf_hash: The leaf hash from the proof
        proof_bytes: The binary proof data
        expected_root: The expected BMT root
        
    Returns:
        True if proof is valid
    """
    # First verify that the leaf hash matches the symbol
    expected_leaf = hash_symbol(symbol)
    if leaf_hash != expected_leaf:
        print(f"[!] Leaf hash mismatch!")
        print(f"   Expected: 0x{expected_leaf.hex()[:16]}...")
        print(f"   Got:      0x{leaf_hash.hex()[:16]}...")
        return False
    
    # Parse proof bytes
    proof = MerkleTree.parse_proof_bytes(proof_bytes)
    
    # Verify the Merkle proof
    return MerkleTree.verify_proof(leaf_hash, proof, expected_root)


def format_time(ms: float) -> str:
    """Format time with < prefix if under threshold."""
    if ms < 1:
        return "< 1 ms"
    elif ms < 20:
        return f"< 20 ms"
    else:
        return f"{ms:.1f} ms"


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="SODS PoC v0.1 — Verify behavioral patterns in Ethereum blocks",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python src/verifier.py --symbol LP+ --block 5000000 --local
  python src/verifier.py --symbol Tf --block 5000000 --local
        """
    )
    
    parser.add_argument(
        "--symbol", "-s",
        required=True,
        help="Behavioral symbol to verify (e.g., LP+, Tf)"
    )
    
    parser.add_argument(
        "--block", "-b",
        type=int,
        default=TARGET_BLOCK,
        help=f"Block number (default: {TARGET_BLOCK})"
    )
    
    parser.add_argument(
        "--local", "-l",
        action="store_true",
        help="Verify using local proof files (required for PoC)"
    )
    
    parser.add_argument(
        "--verbose", "-v",
        action="store_true",
        help="Show detailed verification info"
    )
    
    args = parser.parse_args()
    
    if not args.local:
        print("[ERROR] --local flag is required for PoC verification")
        print("   (P2P verification not implemented in v0.1)")
        return 1
    
    print("\n" + "="*60)
    print("SODS PoC v0.1 - Proof Verifier")
    print("="*60 + "\n")
    
    try:
        # Start timing
        start_time = time.perf_counter()
        
        # Load BMR
        if args.verbose:
            print("[*] Loading BMR...")
        bmr = load_bmr()
        
        # Load proof
        if args.verbose:
            print(f"[*] Loading proof for '{args.symbol}'...")
        leaf_hash, leaf_index, proof_bytes, proof_size = load_proof(args.symbol, args.block)
        
        # Verify
        if args.verbose:
            print("[*] Verifying Merkle proof...")
        is_valid = verify_proof(args.symbol, leaf_hash, proof_bytes, bmr)
        
        elapsed_ms = (time.perf_counter() - start_time) * 1000
        
        # Output results
        print()
        if is_valid:
            print(f"[+] Verified: {args.symbol} exists in block {args.block}")
        else:
            print(f"[-] Failed: {args.symbol} NOT verified in block {args.block}")
        
        print(f"   Proof size: {proof_size} bytes")
        print(f"   Verification time: {format_time(elapsed_ms)}")
        print(f"   Cost: $0.00")
        
        if args.verbose:
            print(f"\n   Details:")
            print(f"   BMR: 0x{bmr.hex()}")
            print(f"   Leaf index: {leaf_index}")
            print(f"   Proof path length: {len(MerkleTree.parse_proof_bytes(proof_bytes))} nodes")
        
        print()
        
        return 0 if is_valid else 1
        
    except FileNotFoundError as e:
        print(f"[ERROR] {e}")
        return 1
    except Exception as e:
        print(f"[ERROR] {e}")
        if args.verbose:
            import traceback
            traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
