//! Causal Merkle Tree implementation.
//!
//! Reconstructs transaction causality by grouping symbols by origin and nonce-ordering them.
//! This enables proving that a sequence of events (e.g. Tf -> Sw -> Tf) happened in a specific
//! causal order within the same context (EOA or Contract Trace), rather than just appearing
//! in the block in that order.


use sha2::{Digest, Sha256};

use crate::symbol::BehavioralSymbol;
use crate::proof::Proof;

/// A Merkle tree that proves causal relationships between events.
#[derive(Debug, Clone)]
pub struct CausalMerkleTree {
    /// Symbols sorted by (from, nonce, call_sequence)
    symbols: Vec<BehavioralSymbol>,
    /// Tree layers
    layers: Vec<Vec<[u8; 32]>>,
    /// Root hash
    root: [u8; 32],
}

impl CausalMerkleTree {
    /// Build a new Causal Merkle Tree from a list of symbols.
    pub fn new(mut symbols: Vec<BehavioralSymbol>) -> Self {
        // Sort by causality: From Address -> Nonce -> Call Sequence
        // This groups all events from the same actor together in chronological order.
        symbols.sort_by(|a, b| {
            a.from.cmp(&b.from)
                .then(a.nonce.cmp(&b.nonce))
                .then(a.call_sequence.cmp(&b.call_sequence))
        });

        if symbols.is_empty() {
             let root = Sha256::digest([]).into();
            return Self { symbols, layers: vec![], root };
        }

        // Compute leaf hashes
        let leaves: Vec<[u8; 32]> = symbols.iter().map(|s| s.leaf_hash()).collect();

        // Build standard binary Merkle tree over the causal sorting
        let (layers, root) = Self::build_tree(leaves);

        Self {
            symbols,
            layers,
            root,
        }
    }

   /// Build the Merkle tree layers from leaves to root.
    fn build_tree(leaves: Vec<[u8; 32]>) -> (Vec<Vec<[u8; 32]>>, [u8; 32]) {
        if leaves.is_empty() {
            return (vec![], Sha256::digest([]).into());
        }

        if leaves.len() == 1 {
            return (vec![leaves.clone()], leaves[0]);
        }

        let mut layers = vec![leaves];

        loop {
            let current = layers.last().unwrap();

            if current.len() == 1 {
                break;
            }

            let mut next_layer = Vec::with_capacity((current.len() + 1) / 2);

            for i in (0..current.len()).step_by(2) {
                let left = current[i];
                let right = if i + 1 < current.len() { current[i + 1] } else { left };

                let mut hasher = Sha256::new();
                hasher.update(left);
                hasher.update(right);
                next_layer.push(hasher.finalize().into());
            }

            layers.push(next_layer);
        }

        let root = layers.last().unwrap()[0];
        (layers, root)
    }

    pub fn root(&self) -> [u8; 32] {
        self.root
    }

    /// Generate a proof for a specific symbol in the causal tree.
    pub fn generate_proof(&self, symbol: &str, log_index: u32) -> Option<Proof> {
        // Find by simple match - in CMT, log_index isn't the primary sort key anymore,
        // but it is still unique per block.
        let leaf_index = self.symbols.iter().position(|s| {
            s.symbol() == symbol && s.log_index() == log_index
        })?;

        self.generate_proof_by_index(leaf_index)
    }

    pub fn generate_proof_by_index(&self, leaf_index: usize) -> Option<Proof> {
        if leaf_index >= self.symbols.len() || self.layers.is_empty() {
            return None;
        }

        let symbol = &self.symbols[leaf_index];
        let leaf_hash = self.layers[0][leaf_index];

        let mut path = Vec::new();
        let mut directions = Vec::new();
        let mut idx = leaf_index;

        for layer in self.layers.iter().take(self.layers.len().saturating_sub(1)) {
            if idx % 2 == 0 {
                let sibling_idx = idx + 1;
                path.push(if sibling_idx < layer.len() { layer[sibling_idx] } else { layer[idx] });
                directions.push(true);
            } else {
                let sibling_idx = idx - 1;
                path.push(layer[sibling_idx]);
                directions.push(false);
            }
            idx /= 2;
        }

        Some(Proof {
            symbol: symbol.symbol().to_string(),
            log_index: symbol.log_index(),
            leaf_hash,
            path,
            directions,
        })
    }
    
    /// Find a sequence of symbols causal-ordered.
    pub fn find_causal_sequence(&self, target_symbols: &[&str]) -> Option<Vec<&BehavioralSymbol>> {
        // Naive search for simplified PoC
        // Real implementation would look for contiguous blocks in `self.symbols`
        // which match the sequence and have same origin + sequential nonce.
        
        let mut current_idx = 0;
        let mut potential_match = Vec::new();
        
        // This is O(N*M) but N is small (block symbols)
        while current_idx < self.symbols.len() {
            let start_sym = &self.symbols[current_idx];
            
             // Check if start symbol matches first target
            if start_sym.symbol() == target_symbols[0] {
                potential_match.clear();
                potential_match.push(start_sym);
                
                let mut match_ptr = 1;
                let mut search_ptr = current_idx + 1;
                
                while match_ptr < target_symbols.len() && search_ptr < self.symbols.len() {
                    let next_sym = &self.symbols[search_ptr];
                    
                    // MUST have same origin
                    if next_sym.from != start_sym.from {
                        break;
                    }
                    
                    // MUST match target symbol
                    if next_sym.symbol() == target_symbols[match_ptr] {
                        // MUST be (nonce + 1) OR (same nonce && sequence + 1)
                        let is_next_nonce = next_sym.nonce == potential_match.last().unwrap().nonce + 1;
                        let is_next_seq = next_sym.nonce == potential_match.last().unwrap().nonce 
                                          && next_sym.call_sequence > potential_match.last().unwrap().call_sequence;
                                          
                        if is_next_nonce || is_next_seq {
                            potential_match.push(next_sym);
                            match_ptr += 1;
                        }
                    }
                    
                    search_ptr += 1;
                }
                
                if potential_match.len() == target_symbols.len() {
                    return Some(potential_match);
                }
            }
            current_idx += 1;
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers_core::types::{Address, H256};

    fn mock_sym(symbol: &str, from: u64, nonce: u64, log_index: u32) -> BehavioralSymbol {
        BehavioralSymbol::new(symbol, log_index)
            .with_context(Address::from_low_u64_be(from), Address::zero(), 0.into(), None)
            .with_causality(H256::random(), nonce, 0)
    }

    #[test]
    fn test_causal_sorting() {
        let sym1 = mock_sym("Tf", 1, 10, 0);
        let sym2 = mock_sym("Dep", 1, 5, 1); // Scrambled order
        let sym3 = mock_sym("Sw", 2, 1, 2);  // Different actor

        let tree = CausalMerkleTree::new(vec![sym1.clone(), sym2.clone(), sym3.clone()]);
        
        // Should be sorted by Origin -> Nonce
        // Origin 1, Nonce 5 (Dep)
        // Origin 1, Nonce 10 (Tf)
        // Origin 2, Nonce 1 (Sw)
        
        assert_eq!(tree.symbols[0].symbol(), "Dep");
        assert_eq!(tree.symbols[1].symbol(), "Tf");
        assert_eq!(tree.symbols[2].symbol(), "Sw");
    }

    #[test]
    fn test_causal_proof_sandwich() {
        // Victim (Actor 1) - Just noise
        let v1 = mock_sym("Tf", 1, 100, 0);
        
        // Attacker (Actor 2) - Executing Sandwich
        let a1 = mock_sym("Tf", 2, 50, 1); // Frontrun
        let a2 = mock_sym("Sw", 2, 51, 2); // Swap
        let a3 = mock_sym("Tf", 2, 52, 3); // Backrun
        
        // Noise (Actor 2) - Later
        let a4 = mock_sym("Wdw", 2, 99, 4);

        let symbols = vec![v1, a1.clone(), a2.clone(), a3.clone(), a4];
        let tree = CausalMerkleTree::new(symbols);
        let root = tree.root();

        // 1. Generate Proofs
        let sequence = vec![a1, a2, a3];
        let mut proofs = Vec::new();
        for sym in &sequence {
             proofs.push(tree.generate_proof(sym.symbol(), sym.log_index()).unwrap());
        }

        let proof = crate::proof::CausalProof {
            root,
            symbols: sequence.clone(),
            proofs,
        };

        // 2. Verify Valid Sequence
        assert!(proof.verify(&root));
    }

    #[test]
    fn test_causal_proof_rejects_gap() {
        let a1 = mock_sym("Tf", 2, 50, 0);
        let a2 = mock_sym("Sw", 2, 52, 1); // Gap! Nonce 51 missing
        
        let tree = CausalMerkleTree::new(vec![a1.clone(), a2.clone()]);
        let root = tree.root();
        
        let proof = crate::proof::CausalProof {
            root,
            symbols: vec![a1.clone(), a2.clone()],
            proofs: vec![
                tree.generate_proof("Tf", 0).unwrap(),
                tree.generate_proof("Sw", 1).unwrap()
            ],
        };
        
        assert!(!proof.verify(&root));
    }
}
