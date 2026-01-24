//! Tests for Merkle path ordering consistency between Rust and on-chain verification.

use sods_core::{BehavioralMerkleTree, BehavioralSymbol};

#[test]
fn test_onchain_proof_has_is_left_path() {
    let symbols = vec![
        BehavioralSymbol::new("Tf", 0),
        BehavioralSymbol::new("Sw", 1),
    ];
    let bmt = BehavioralMerkleTree::new_keccak(symbols.clone());
    let matched = vec![&symbols[0]];
    
    let proof = bmt.generate_onchain_proof(&matched, 1, 100, None, 0).unwrap();
    
    // is_left_path must be populated
    assert!(!proof.is_left_path.is_empty(), "is_left_path should not be empty");
    // Length must match merkle_path
    assert_eq!(
        proof.merkle_path.len(), 
        proof.is_left_path.len(), 
        "merkle_path and is_left_path must have same length"
    );
}

#[test]
fn test_directions_match_proof_struct() {
    let symbols = vec![
        BehavioralSymbol::new("Tf", 0),
        BehavioralSymbol::new("Sw", 1),
        BehavioralSymbol::new("Dep", 2),
        BehavioralSymbol::new("Wdw", 3),
    ];
    let bmt = BehavioralMerkleTree::new_keccak(symbols.clone());
    
    // Generate on-chain proof
    let matched = vec![&symbols[1]]; // Sw at index 1
    let onchain_proof = bmt.generate_onchain_proof(&matched, 1, 100, None, 0).unwrap();
    
    // Generate standard proof for same symbol
    let std_proof = bmt.generate_proof("Sw", 1).unwrap();
    
    // Directions should match (though hashing differs)
    assert_eq!(
        std_proof.directions.len(),
        onchain_proof.is_left_path.len(),
        "Direction arrays must have same length"
    );
    assert_eq!(
        std_proof.directions,
        onchain_proof.is_left_path,
        "Direction values must match"
    );
}

#[test]
fn test_single_leaf_tree_no_path() {
    let symbols = vec![BehavioralSymbol::new("Tf", 0)];
    let bmt = BehavioralMerkleTree::new_keccak(symbols.clone());
    let matched = vec![&symbols[0]];
    
    let proof = bmt.generate_onchain_proof(&matched, 1, 100, None, 0).unwrap();
    
    // Single leaf tree has no siblings
    assert!(proof.merkle_path.is_empty());
    assert!(proof.is_left_path.is_empty());
}

#[test]
fn test_calldata_includes_is_left_path() {
    let syms = vec![
        BehavioralSymbol::new("Tf", 0),
        BehavioralSymbol::new("Sw", 1),
        BehavioralSymbol::new("Dep", 2),
    ];
    let bmt = BehavioralMerkleTree::new_keccak(syms.clone());
    let matched = vec![&syms[0]];
    
    let proof = bmt.generate_onchain_proof(&matched, 11155111, 100, None, 0).unwrap();
    let calldata = proof.to_calldata();
    
    // v3 calldata should be larger due to is_left_path array
    // Minimum: 7 fixed slots (224) + dynamic arrays
    assert!(calldata.len() > 256, "Calldata should include is_left_path array");
    assert_eq!(calldata.len() % 32, 0, "ABI encoding must be 32-byte aligned");
}
