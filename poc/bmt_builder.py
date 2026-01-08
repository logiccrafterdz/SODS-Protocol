"""
SODS PoC v0.1 — Behavioral Merkle Tree Builder

Fetches logs for a specific block from Sepolia via Infura,
parses them into behavioral symbols, builds a Merkle tree,
and saves the BMR (root) and proofs.

Usage:
    python src/bmt_builder.py

Outputs:
    - bmr.bin: 32-byte Behavioral Merkle Root
    - proofs/<symbol>_<block>.proof: Binary proof files
"""

import json
import sys
import time
from pathlib import Path
from typing import Dict, List, Tuple, Optional

import requests

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent))

from config import (
    INFURA_RPC_URL,
    INFURA_PROJECT_ID,
    TARGET_BLOCK,
    SYMBOL_REGISTRY,
    BMR_FILE,
    PROOFS_DIR,
)
from merkle import MerkleTree, hash_symbol


def fetch_block_logs(block_number: int) -> List[Dict]:
    """
    Fetch all logs for a specific block via eth_getLogs.
    
    Args:
        block_number: The block number to fetch logs for
        
    Returns:
        List of log objects from the RPC response
        
    Raises:
        RuntimeError: If RPC call fails
    """
    if not INFURA_PROJECT_ID or INFURA_PROJECT_ID == "your_project_id_here":
        raise RuntimeError(
            "Infura Project ID not configured!\n"
            "1. Copy .env.example to .env\n"
            "2. Add your Infura Project ID\n"
            "Get a free key at: https://infura.io"
        )
    
    block_hex = hex(block_number)
    
    payload = {
        "jsonrpc": "2.0",
        "method": "eth_getLogs",
        "params": [{
            "fromBlock": block_hex,
            "toBlock": block_hex,
        }],
        "id": 1
    }
    
    print(f"[*] Fetching logs for block #{block_number} from Sepolia...")
    
    try:
        response = requests.post(
            INFURA_RPC_URL,
            json=payload,
            headers={"Content-Type": "application/json"},
            timeout=30
        )
        response.raise_for_status()
        
        result = response.json()
        
        if "error" in result:
            raise RuntimeError(f"RPC Error: {result['error']}")
        
        logs = result.get("result", [])
        print(f"   Retrieved {len(logs)} logs")
        return logs
        
    except requests.RequestException as e:
        raise RuntimeError(f"Failed to fetch logs: {e}")


def parse_logs_to_symbols(logs: List[Dict]) -> List[Tuple[str, int, Dict]]:
    """
    Parse event logs into behavioral symbols.
    
    Args:
        logs: List of log objects from eth_getLogs
        
    Returns:
        List of (symbol, log_index, metadata) tuples, sorted deterministically
    """
    symbols = []
    
    for log in logs:
        # Get the event signature (topic[0])
        topics = log.get("topics", [])
        if not topics:
            continue
        
        topic0 = topics[0].lower()
        
        # Look up symbol
        symbol = SYMBOL_REGISTRY.get(topic0)
        if not symbol:
            continue
        
        # Extract log index (hex string → int)
        log_index = int(log.get("logIndex", "0x0"), 16)
        
        # Metadata for reference (not used in minimal mode)
        metadata = {
            "address": log.get("address", ""),
            "topics": topics,
            "data": log.get("data", ""),
            "transactionHash": log.get("transactionHash", ""),
        }
        
        symbols.append((symbol, log_index, metadata))
    
    # Sort deterministically: primary by log_index, secondary by symbol
    symbols.sort(key=lambda x: (x[1], x[0]))
    
    return symbols


def build_bmt(symbols: List[Tuple[str, int, Dict]]) -> Tuple[bytes, MerkleTree, Dict[str, int]]:
    """
    Build Behavioral Merkle Tree from symbols.
    
    Uses BMT-Minimal mode: leaf = SHA256(symbol.encode('utf-8'))
    
    Args:
        symbols: List of (symbol, log_index, metadata) tuples
        
    Returns:
        Tuple of (root_hash, merkle_tree, symbol_indices)
        where symbol_indices maps symbol → first occurrence index
    """
    if not symbols:
        print("[!] No matching symbols found in block")
        empty_root = MerkleTree([]).root
        return empty_root, MerkleTree([]), {}
    
    # Hash each symbol to create leaves
    leaves = []
    symbol_indices: Dict[str, int] = {}  # Track first occurrence of each symbol
    
    for idx, (symbol, _log_index, _metadata) in enumerate(symbols):
        leaf_hash = hash_symbol(symbol)
        leaves.append(leaf_hash)
        
        # Store first occurrence index for each unique symbol
        if symbol not in symbol_indices:
            symbol_indices[symbol] = idx
    
    # Build Merkle tree
    tree = MerkleTree(leaves)
    
    return tree.root, tree, symbol_indices


def save_outputs(
    root: bytes,
    tree: MerkleTree,
    symbol_indices: Dict[str, int],
    block_number: int
) -> None:
    """
    Save BMR and proof files.
    
    Args:
        root: 32-byte Merkle root
        tree: MerkleTree object
        symbol_indices: Map of symbol → leaf index
        block_number: Block number for filenames
    """
    # Save BMR
    BMR_FILE.write_bytes(root)
    print(f"[+] Saved BMR to {BMR_FILE}")
    print(f"   Root: 0x{root.hex()}")
    
    # Save proofs for each unique symbol
    for symbol, leaf_index in symbol_indices.items():
        proof_bytes = tree.get_proof_bytes(leaf_index)
        leaf_hash = tree.leaves[leaf_index]
        
        # Proof file format:
        # - 32 bytes: leaf hash
        # - 4 bytes: leaf index (big-endian)
        # - N bytes: proof data
        proof_data = leaf_hash + leaf_index.to_bytes(4, 'big') + proof_bytes
        
        # Sanitize symbol for filename (replace + with _plus)
        safe_symbol = symbol.replace("+", "_plus").replace("-", "_minus")
        proof_file = PROOFS_DIR / f"{safe_symbol}_{block_number}.proof"
        proof_file.write_bytes(proof_data)
        
        print(f"   Proof: {proof_file.name} ({len(proof_data)} bytes)")


def print_summary(symbols: List[Tuple[str, int, Dict]], root: bytes, elapsed: float) -> None:
    """Print build summary."""
    # Count symbols
    symbol_counts: Dict[str, int] = {}
    for symbol, _, _ in symbols:
        symbol_counts[symbol] = symbol_counts.get(symbol, 0) + 1
    
    print("\n" + "="*60)
    print("BEHAVIORAL MERKLE TREE - BUILD COMPLETE")
    print("="*60)
    print(f"   Block:        #{TARGET_BLOCK}")
    print(f"   Chain:        Sepolia (testnet)")
    print(f"   Total Events: {len(symbols)}")
    print(f"   Unique Symbols:")
    for sym, count in sorted(symbol_counts.items()):
        print(f"      • {sym}: {count} occurrences")
    print(f"   BMT Root:     0x{root.hex()[:16]}...")
    print(f"   Build Time:   {elapsed*1000:.1f} ms")
    print(f"   Cost:         $0.00 (Infura free tier)")
    print("="*60)


def main():
    """Main entry point."""
    print("\n" + "="*60)
    print("SODS PoC v0.1 - BMT Builder")
    print("="*60 + "\n")
    
    start_time = time.perf_counter()
    
    try:
        # Step 1: Fetch logs
        logs = fetch_block_logs(TARGET_BLOCK)
        
        # Step 2: Parse to symbols
        print("\n[*] Parsing logs to behavioral symbols...")
        symbols = parse_logs_to_symbols(logs)
        print(f"   Found {len(symbols)} matching events")
        
        # Step 3: Build BMT
        print("\n[*] Building Behavioral Merkle Tree...")
        root, tree, symbol_indices = build_bmt(symbols)
        
        # Step 4: Save outputs
        print("\n[*] Saving outputs...")
        save_outputs(root, tree, symbol_indices, TARGET_BLOCK)
        
        elapsed = time.perf_counter() - start_time
        print_summary(symbols, root, elapsed)
        
        return 0
        
    except Exception as e:
        print(f"\n[ERROR] {e}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
