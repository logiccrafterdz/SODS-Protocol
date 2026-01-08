"""
SODS PoC v0.1 — Merkle Tree Implementation

A minimal, deterministic Merkle tree using SHA-256.
Follows the BMT construction algorithm from SODS RFC v0.2 Section 4.
"""

import hashlib
from typing import List, Optional, Tuple


class MerkleTree:
    """
    Binary Merkle tree with SHA-256 hashing.
    
    Properties:
        - Deterministic: same leaves → same root
        - Proof generation: O(log n) path from leaf to root
        - Verification: O(log n) hash computations
    """
    
    def __init__(self, leaves: List[bytes]):
        """
        Build a Merkle tree from a list of leaf hashes.
        
        Args:
            leaves: List of 32-byte SHA-256 hashes (pre-hashed leaves)
        """
        self.leaves = leaves
        self.layers: List[List[bytes]] = []
        self.root: bytes = b''
        
        if leaves:
            self._build()
        else:
            # Empty tree: root = SHA256('')
            self.root = hashlib.sha256(b'').digest()
    
    def _build(self) -> None:
        """Build the Merkle tree from leaves to root."""
        # Layer 0 = leaves
        current_layer = self.leaves.copy()
        self.layers.append(current_layer)
        
        # Build up to root
        while len(current_layer) > 1:
            next_layer = []
            
            for i in range(0, len(current_layer), 2):
                left = current_layer[i]
                
                # If odd number of nodes, duplicate the last one
                if i + 1 < len(current_layer):
                    right = current_layer[i + 1]
                else:
                    right = left
                
                # Parent = H(left || right)
                parent = hashlib.sha256(left + right).digest()
                next_layer.append(parent)
            
            self.layers.append(next_layer)
            current_layer = next_layer
        
        # Root is the single node in the top layer
        self.root = self.layers[-1][0] if self.layers else hashlib.sha256(b'').digest()
    
    def get_proof(self, leaf_index: int) -> List[Tuple[bytes, str]]:
        """
        Generate Merkle proof for a leaf at given index.
        
        Args:
            leaf_index: 0-based index of the leaf
            
        Returns:
            List of (sibling_hash, direction) tuples where direction is 'L' or 'R'
            indicating whether the sibling is on the left or right.
            
        Raises:
            IndexError: If leaf_index is out of bounds
        """
        if not self.leaves:
            return []
        
        if leaf_index < 0 or leaf_index >= len(self.leaves):
            raise IndexError(f"Leaf index {leaf_index} out of bounds (0-{len(self.leaves)-1})")
        
        proof = []
        idx = leaf_index
        
        # Traverse from leaf layer (0) up to second-to-last layer
        for layer in self.layers[:-1]:
            # Get sibling index
            if idx % 2 == 0:
                # Current is left child, sibling is right
                sibling_idx = idx + 1
                direction = 'R'
            else:
                # Current is right child, sibling is left
                sibling_idx = idx - 1
                direction = 'L'
            
            # Handle odd-length layers (last node has no sibling, so duplicate)
            if sibling_idx >= len(layer):
                sibling_idx = idx  # Use self as sibling (duplication case)
            
            sibling_hash = layer[sibling_idx]
            proof.append((sibling_hash, direction))
            
            # Move to parent index
            idx = idx // 2
        
        return proof
    
    @staticmethod
    def verify_proof(
        leaf_hash: bytes,
        proof: List[Tuple[bytes, str]],
        expected_root: bytes
    ) -> bool:
        """
        Verify a Merkle proof.
        
        Args:
            leaf_hash: The hash of the leaf being verified
            proof: List of (sibling_hash, direction) tuples
            expected_root: The expected Merkle root
            
        Returns:
            True if proof is valid, False otherwise
        """
        current = leaf_hash
        
        for sibling_hash, direction in proof:
            if direction == 'L':
                # Sibling is on the left
                current = hashlib.sha256(sibling_hash + current).digest()
            else:
                # Sibling is on the right
                current = hashlib.sha256(current + sibling_hash).digest()
        
        return current == expected_root
    
    def get_proof_bytes(self, leaf_index: int) -> bytes:
        """
        Get proof as a compact binary format.
        
        Format:
            - 1 byte: number of proof elements
            - For each element: 1 byte direction (0=L, 1=R) + 32 bytes hash
            
        Returns:
            Binary proof data
        """
        proof = self.get_proof(leaf_index)
        
        result = bytes([len(proof)])
        for sibling_hash, direction in proof:
            dir_byte = 0 if direction == 'L' else 1
            result += bytes([dir_byte]) + sibling_hash
        
        return result
    
    @staticmethod
    def parse_proof_bytes(proof_bytes: bytes) -> List[Tuple[bytes, str]]:
        """
        Parse binary proof format back to list of tuples.
        
        Args:
            proof_bytes: Binary proof data
            
        Returns:
            List of (sibling_hash, direction) tuples
        """
        if not proof_bytes:
            return []
        
        num_elements = proof_bytes[0]
        proof = []
        offset = 1
        
        for _ in range(num_elements):
            direction = 'L' if proof_bytes[offset] == 0 else 'R'
            sibling_hash = proof_bytes[offset + 1:offset + 33]
            proof.append((sibling_hash, direction))
            offset += 33
        
        return proof


def hash_leaf(data: bytes) -> bytes:
    """Hash leaf data with SHA-256."""
    return hashlib.sha256(data).digest()


def hash_symbol(symbol: str) -> bytes:
    """Hash a symbol string for use as a Merkle leaf."""
    return hash_leaf(symbol.encode('utf-8'))
