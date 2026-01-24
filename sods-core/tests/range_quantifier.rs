use sods_core::pattern::BehavioralPattern;
use sods_core::symbol::BehavioralSymbol;

fn mock_sym(s: &str, idx: u32) -> BehavioralSymbol {
    BehavioralSymbol::new(s, idx)
}

#[test]
fn test_greedy_consumption() {
    let symbols = vec![
        mock_sym("Sw", 0),
        mock_sym("Sw", 1), 
        mock_sym("Sw", 2),
        mock_sym("Tf", 3)
    ];
    let pattern = BehavioralPattern::parse("Sw{1,3}").unwrap();
    let result = pattern.matches(&symbols, None).unwrap();
    // In greedy mode, Sw{1,3} should consume ALL 3 Swaps if they are consecutive.
    // However, PatternStep::Range / AtLeast adds the symbols to matched_sequence.
    // Let's verify the count of symbols in the sequence.
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].log_index(), 0);
    assert_eq!(result[2].log_index(), 2);
}

#[test]
fn test_exact_range() {
    let symbols = vec![mock_sym("Sw", 0), mock_sym("Sw", 1)];
    let pattern = BehavioralPattern::parse("Sw{2,2}").unwrap();
    let result = pattern.matches(&symbols, None);
    assert!(result.is_some());
    assert_eq!(result.unwrap().len(), 2);
}

#[test]
fn test_insufficient_range() {
    let symbols = vec![mock_sym("Sw", 0)];
    let pattern = BehavioralPattern::parse("Sw{2,5}").unwrap();
    assert!(pattern.matches(&symbols, None).is_none());
}

#[test]
fn test_range_boundary_limit() {
    let symbols = vec![
        mock_sym("Sw", 0),
        mock_sym("Sw", 1), 
        mock_sym("Sw", 2),
        mock_sym("Sw", 3),
        mock_sym("Tf", 4)
    ];
    let pattern = BehavioralPattern::parse("Sw{1,2}").unwrap();
    let result = pattern.matches(&symbols, None).unwrap();
    // Should stop at 2 even if more are available
    assert_eq!(result.len(), 2);
    assert_eq!(result[1].log_index(), 1);
}

#[test]
fn test_at_least_greedy() {
    let symbols = vec![
        mock_sym("Sw", 0),
        mock_sym("Sw", 1), 
        mock_sym("Sw", 2),
        mock_sym("Tf", 3)
    ];
    let pattern = BehavioralPattern::parse("Sw{2,}").unwrap();
    let result = pattern.matches(&symbols, None).unwrap();
    // Sw{2,} should consume ALL 3 Swaps
    assert_eq!(result.len(), 3);
}

#[test]
fn test_sequential_greedy_patterns() {
    let symbols = vec![
        mock_sym("Tf", 0),
        mock_sym("Sw", 1), 
        mock_sym("Sw", 2),
        mock_sym("Sw", 3),
        mock_sym("Tf", 4)
    ];
    // Pattern: Tf -> Sw{2,5} -> Tf
    let pattern = BehavioralPattern::parse("Tf -> Sw{2,5} -> Tf").unwrap();
    let result = pattern.matches(&symbols, None).unwrap();
    assert_eq!(result.len(), 5); // 1 Tf + 3 Sw + 1 Tf
    assert_eq!(result[0].symbol(), "Tf");
    assert_eq!(result[4].symbol(), "Tf");
    assert_eq!(result[4].log_index(), 4);
}
