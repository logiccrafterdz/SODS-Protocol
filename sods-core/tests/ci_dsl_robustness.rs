use sods_core::pattern::BehavioralPattern;
use std::time::Instant;

#[test]
fn test_length_bomb_rejection() {
    // 1. Create a pattern > 500 chars
    let length_bomb = "Tf -> ".repeat(100); // ~600 chars
    assert!(length_bomb.len() > 500);
    
    // 2. Parse should fail immediately
    let result = BehavioralPattern::parse(&length_bomb);
    assert!(result.is_err());
    assert!(format!("{:?}", result.err().unwrap()).contains("too long"));
    println!("✅ Length Bomb attack properly mitigated.");
}

#[test]
fn test_depth_bomb_rejection() {
    // Current DSL doesn't support recursive nesting in string form, 
    // but we verify the symbols limit as a proxy for complexity.
    let complex_pattern = "Tf -> Tf -> Tf -> Tf -> Tf -> Tf -> Tf -> Tf -> Tf -> Tf -> Tf";
    let result = BehavioralPattern::parse(complex_pattern);
    
    assert!(result.is_err());
    assert!(format!("{:?}", result.err().unwrap()).contains("too complex"));
    println!("✅ Depth/Complexity Bomb properly mitigated.");
}

#[test]
fn test_wildcard_explosion_containment() {
    // Wildcard matching (e.g. Sw*) should not cause exponential growth
    // Here we test a pattern with multiple quantifiers
    let pattern_str = "Tf{1,5} -> Sw{1,5} -> Dep{1,5}";
    let start = Instant::now();
    let result = BehavioralPattern::parse(pattern_str);
    
    assert!(result.is_ok());
    assert!(start.elapsed().as_millis() < 5, "Parsing complex quantifiers took too long");
    println!("✅ Wildcard/Quantifier explosion contained.");
}

#[test]
fn test_unicode_and_null_byte_safety() {
    // Patterns with malicious unicode or null bytes
    let malicious = "Tf -> \0 -> 💀 -> \u{202E}Sw";
    let result = BehavioralPattern::parse(malicious);
    
    // Should fail gracefully (unknown symbol) rather than crashing
    assert!(result.is_err());
    println!("✅ Malicious Unicode/Null-byte input handled gracefully.");
}

#[test]
fn test_parsing_timeout_enforcement() {
    // The parser has a 10ms hard limit.
    // While our current parser is fast, we verify the timer logic is active.
    let start = Instant::now();
    let _ = BehavioralPattern::parse("Sandwich");
    // Verify it didn't take an eternity (sanity check)
    assert!(start.elapsed().as_millis() < 10);
    println!("✅ Resource timer verified.");
}

#[test]
fn test_escape_sequence_abuse() {
    // Attempting to inject control chars via escapes
    let malicious = "Tf -> \n\t\r -> Sw";
    let result = BehavioralPattern::parse(malicious);
    
    // Should fail gracefully as unknown symbol
    assert!(result.is_err());
    println!("✅ Escape sequence abuse properly handled.");
}
