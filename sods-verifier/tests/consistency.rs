use sods_core::{BehavioralMerkleTree, BehavioralSymbol};

#[test]
fn test_full_vs_incremental_root_parity() {
    let symbols = vec![
        BehavioralSymbol::new("Tf", 100),
        BehavioralSymbol::new("Sw", 105),
        BehavioralSymbol::new("Tf", 110),
    ];

    // Mode 1: Full Construction
    let bmt_full = BehavioralMerkleTree::new(symbols.clone());
    let root_full = bmt_full.root();

    // Mode 2: Incremental Construction (Filtered set)
    let bmt_inc = BehavioralMerkleTree::build_incremental(symbols);
    let root_inc = bmt_inc.root();

    assert_eq!(root_full, root_inc, "Full and Incremental roots MUST be identical for parity");
    println!("✅ Cross-Implementation Root Parity Verified.");
}

#[test]
fn test_root_determinism_across_runs() {
    let symbols = || vec![
        BehavioralSymbol::new("Dep", 50),
        BehavioralSymbol::new("Wdw", 60),
    ];

    let root1 = BehavioralMerkleTree::new(symbols()).root();
    let root2 = BehavioralMerkleTree::new(symbols()).root();

    assert_eq!(root1, root2, "BMT roots MUST be deterministic");
}
