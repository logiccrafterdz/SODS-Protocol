use sods_core::pattern::BehavioralPattern;
use sods_core::symbol::BehavioralSymbol;

fn mock_sym(s: &str, idx: u32) -> BehavioralSymbol {
    BehavioralSymbol::new(s, idx)
}

#[test]
fn test_complex_mev_sandwich_greedy() {
    // Scenario: User performs multiple swaps in a single block (multi-hop)
    // Targeted by a sandwich: Tf -> [Sw, Sw, Sw] -> Tf
    let symbols = vec![
        mock_sym("Tf", 0),
        mock_sym("Sw", 1), 
        mock_sym("Sw", 2),
        mock_sym("Sw", 3),
        mock_sym("Tf", 4),
    ];
    
    // Greedy pattern: Sw{2, 5} should capture all middle swaps
    let pattern = BehavioralPattern::parse("Tf -> Sw{2,5} -> Tf").unwrap();
    let result = pattern.matches(&symbols, None).unwrap();
    
    assert_eq!(result.len(), 5);
    assert_eq!(result[0].symbol(), "Tf");
    assert_eq!(result[1].symbol(), "Sw");
    assert_eq!(result[2].symbol(), "Sw");
    assert_eq!(result[3].symbol(), "Sw");
    assert_eq!(result[4].symbol(), "Tf");
}

#[test]
fn test_greedy_at_least_mev() {
    // Scenario: Detecting aggressive high-frequency trading: Sw{5,}
    let symbols = vec![
        mock_sym("Sw", 1),
        mock_sym("Sw", 2),
        mock_sym("Sw", 3),
        mock_sym("Sw", 4),
        mock_sym("Sw", 5),
        mock_sym("Sw", 6),
        mock_sym("Tf", 7),
    ];
    
    let pattern = BehavioralPattern::parse("Sw{5,}").unwrap();
    let result = pattern.matches(&symbols, None).unwrap();
    
    assert_eq!(result.len(), 6); // Should consume all 6 swaps
}
