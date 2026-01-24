#![no_main]
use libfuzzer_sys::fuzz_target;
use sods_core::{BehavioralMerkleTree, BehavioralSymbol};

fuzz_target!(|data: Vec<(String, u32)>| {
    let symbols: Vec<BehavioralSymbol> = data.into_iter()
        .map(|(s, idx)| BehavioralSymbol::new(&s, idx))
        .collect();
    
    let bmt = BehavioralMerkleTree::new(symbols);
    let _root = bmt.root();
});
