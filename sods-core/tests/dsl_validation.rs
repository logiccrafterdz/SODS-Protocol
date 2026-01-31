use sods_core::pattern::BehavioralPattern;

#[test]
fn test_quantifier_limits() {
    // Exact quantifier at limit
    assert!(BehavioralPattern::parse("Tf{1000}").is_ok());
    // Exact quantifier over limit
    assert!(BehavioralPattern::parse("Tf{1001}").is_err());
    
    // Range quantifier at limit
    assert!(BehavioralPattern::parse("Tf{0,1000}").is_ok());
    // Range quantifier min over limit
    assert!(BehavioralPattern::parse("Tf{1001,1001}").is_err());
    // Range quantifier max over limit
    assert!(BehavioralPattern::parse("Tf{0,1001}").is_err());
}

#[test]
fn test_nested_quantifiers_rejected() {
    // Should reject nested quantifiers
    assert!(BehavioralPattern::parse("Tf{2}{3}").is_err());
    assert!(BehavioralPattern::parse("Tf{2}{,5}").is_err());
    assert!(BehavioralPattern::parse("Tf{2}{3,5}").is_err());
}

#[test]
fn test_syntax_errors_and_error_messages() {
    // Unclosed bracket
    let err = BehavioralPattern::parse("Tf{2").unwrap_err().to_string();
    assert!(err.contains("Unclosed quantifier"), "Error was: {}", err);
    assert!(err.contains("expected '}'"), "Error was: {}", err);

    // Unmatched closing bracket
    let err = BehavioralPattern::parse("Tf}").unwrap_err().to_string();
    assert!(err.contains("Unmatched '}'"), "Error was: {}", err);

    // Empty pattern segment
    let err = BehavioralPattern::parse("Tf -> -> Sw").unwrap_err().to_string();
    assert!(err.contains("Empty pattern segment"), "Error was: {}", err);

    // Invalid symbol name
    let err = BehavioralPattern::parse("Tf$").unwrap_err().to_string();
    assert!(err.contains("Invalid symbol name"), "Error was: {}", err);
}

#[test]
fn test_complex_pattern_complexity_limit() {
    // Max symbols is 10
    let complex = "Tf -> Sw -> Tf -> Sw -> Tf -> Sw -> Tf -> Sw -> Tf -> Sw"; // 10 symbols
    assert!(BehavioralPattern::parse(complex).is_ok());

    let too_complex = "Tf -> Sw -> Tf -> Sw -> Tf -> Sw -> Tf -> Sw -> Tf -> Sw -> Tf"; // 11 symbols
    assert!(BehavioralPattern::parse(too_complex).is_err());
}

#[test]
fn test_pattern_length_limit() {
    let mut long_pattern = String::new();
    for _ in 0..100 {
        long_pattern.push_str("LongSymbolName -> ");
    }
    long_pattern.push_str("Tf");
    // Ensure it exceeds 500
    assert!(long_pattern.len() > 500, "Length was: {}", long_pattern.len());
    assert!(BehavioralPattern::parse(&long_pattern).is_err());
}

#[test]
fn test_invalid_range_quantifier() {
    // max < min
    let err = BehavioralPattern::parse("Tf{5,2}").unwrap_err().to_string();
    assert!(err.contains("must be >= min"), "Error was: {}", err);
}

#[test]
fn test_malformed_conditions() {
    assert!(BehavioralPattern::parse("Tf where unknown == condition").is_err());
    assert!(BehavioralPattern::parse("Tf where value > not_a_number").is_err());
}

#[test]
fn test_trailing_garbage_after_quantifier() {
    // Reject patterns like "Tf{2}extra"
    assert!(BehavioralPattern::parse("Tf{2}extra").is_err());
}
