use sods_core::{BehavioralMerkleTree, BehavioralSymbol};
use sha2::{Digest, Sha256};

#[test]
fn test_empty_block_correctness() {
    let symbols: Vec<BehavioralSymbol> = vec![];
    let bmt = BehavioralMerkleTree::new(symbols);
    
    // Empty tree root MUST be SHA256(b"")
    let expected: [u8; 32] = Sha256::digest([]).into();
    assert_eq!(bmt.root(), expected);
}

#[test]
fn test_single_log_correctness() {
    let sym = BehavioralSymbol::new("Tf", 0);
    let bmt = BehavioralMerkleTree::new(vec![sym.clone()]);
    
    // Single log root MUST be the leaf hash
    assert_eq!(bmt.root(), sym.leaf_hash());
}

#[test]
fn test_ordering_sensitivity() {
    let sym1 = BehavioralSymbol::new("Tf", 10);
    let sym2 = BehavioralSymbol::new("Sw", 20);
    
    // BMT constructor sorts symbols automatically.
    // To test ordering sensitivity, we compare the roots of trees 
    // with different log_indices (representing different causal orders).
    
    let root_12 = BehavioralMerkleTree::new(vec![sym1.clone(), sym2.clone()]).root();
    
    // Swap indices
    let sym1_late = BehavioralSymbol::new("Tf", 30);
    let sym2_early = BehavioralSymbol::new("Sw", 5);
    
    let root_21 = BehavioralMerkleTree::new(vec![sym1_late, sym2_early]).root();
    
    assert_ne!(root_12, root_21, "BMT root MUST be sensitive to log ordering");
}

#[test]
fn test_injection_resistance() {
    let sym1 = BehavioralSymbol::new("Tf", 10);
    let sym2 = BehavioralSymbol::new("Sw", 20);
    let base_root = BehavioralMerkleTree::new(vec![sym1.clone(), sym2.clone()]).root();
    
    // Inject a third log
    let sym3 = BehavioralSymbol::new("Dep", 15);
    let injected_root = BehavioralMerkleTree::new(vec![sym1, sym2, sym3]).root();
    
    assert_ne!(base_root, injected_root, "BMT root MUST change if logs are injected");
}

#[test]
fn test_duplicate_symbols_uniqueness() {
    // Two transfers in the same block at different positions
    let sym1 = BehavioralSymbol::new("Tf", 10);
    let sym2 = BehavioralSymbol::new("Tf", 11);
    
    let bmt = BehavioralMerkleTree::new(vec![sym1, sym2]);
    assert_eq!(bmt.len(), 2);
    
    // Roots of [Tf@10, Tf@11] vs [Tf@10] MUST differ
    assert_ne!(bmt.root(), BehavioralMerkleTree::new(vec![BehavioralSymbol::new("Tf", 10)]).root());
}

#[test]
fn test_substitution_detectability() {
    let sym1 = BehavioralSymbol::new("Tf", 10);
    let sym2 = BehavioralSymbol::new("Sw", 20);
    let base_root = BehavioralMerkleTree::new(vec![sym1.clone(), sym2.clone()]).root();
    
    // Replace Sw with Tf at the same index
    let sym2_alt = BehavioralSymbol::new("Tf", 20);
    let substituted_root = BehavioralMerkleTree::new(vec![sym1, sym2_alt]).root();
    
    assert_ne!(base_root, substituted_root, "BMT root MUST change on symbol substitution");
}
