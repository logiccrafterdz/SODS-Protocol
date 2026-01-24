use sods_zk::prove_behavior;
use sods_core::BehavioralSymbol;

#[test]
fn test_zk_completeness_sandwich_validation() {
    let symbols = vec![
        BehavioralSymbol::new("Tf", 10),
        BehavioralSymbol::new("Sw", 11),
        BehavioralSymbol::new("Tf", 12),
    ];
    
    // Valid sandwich
    let result = prove_behavior(symbols, "Sandwich");
    assert!(result.is_ok());
    
    let receipt = result.unwrap();
    let valid: bool = receipt.journal.decode().unwrap();
    assert!(valid, "ZK Completeness Failure: Failed to prove valid sandwich");
    println!("✅ Completeness: Valid sandwich pattern proven.");
}

#[test]
fn test_zk_completeness_complex_patterns() {
    let symbols = vec![
        BehavioralSymbol::new("AAOp", 100),
        BehavioralSymbol::new("Permit2", 101),
        BehavioralSymbol::new("Sw", 102),
    ];
    
    let result = prove_behavior(symbols, "AAOp -> Permit2 -> Sw");
    assert!(result.is_ok());
    
    let receipt = result.unwrap();
    let valid: bool = receipt.journal.decode().unwrap();
    assert!(valid, "ZK Completeness Failure: Failed to prove composite sequence");
    println!("✅ Completeness: Complex composite sequence proven.");
}

#[test]
fn test_zk_completeness_single_symbol() {
    let symbols = vec![BehavioralSymbol::new("Tf", 50)];
    let result = prove_behavior(symbols, "Tf");
    
    let valid: bool = result.unwrap().journal.decode().unwrap();
    assert!(valid);
    println!("✅ Completeness: Single symbol pattern proven.");
}
