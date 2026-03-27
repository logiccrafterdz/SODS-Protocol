use proptest::prelude::*;
use sods_core::pattern::BehavioralPattern;

proptest! {
    // This test feeds entirely random strings into the pattern parser.
    // The objective is to verify that it NEVER panics or enters an infinite loop,
    // regardless of structure, length, or random character combinations.
    #[test]
    fn test_parser_never_panics_on_garbage(input in "\\PC*") {
        // limit the input to prevent extremely slow timeouts in CI due to massive strings
        let bounded_input = if input.len() > 1000 {
            &input[0..1000]
        } else {
            &input
        };
        
        // The parser should safely return a Result.
        let _result = BehavioralPattern::parse(bounded_input);
        // We only assert that it didn't panic, which is natively guaranteed 
        // if this test reaches the end of the closure.
    }

    // This test specifically tries to hammer the quantifier logic
    // to search for ReDoS (Regular Expression Denial of Service) vectors
    // or out of bounds indices when parsing {min,max}.
    #[test]
    fn test_quantifier_fuzzing(
        symbol in "[a-zA-Z]+",
        min in 0usize..5000,
        max in 0usize..5000,
        has_comma in any::<bool>(),
        is_where in any::<bool>(),
        condition in "[A-Za-z0-9_= ]*"
    ) {
        let pattern_str = if is_where {
            if has_comma {
                format!("{}{{{},{}}} where {}", symbol, min, max, condition)
            } else {
                format!("{}{{{}}} where {}", symbol, min, condition)
            }
        } else {
            if has_comma {
                format!("{}{{{},{}}}", symbol, min, max)
            } else {
                format!("{}{{{}}}", symbol, min)
            }
        };

        let _ = BehavioralPattern::parse(&pattern_str);
    }
}
