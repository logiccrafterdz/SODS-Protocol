use ethers_core::types::U256;
use crate::symbol::BehavioralSymbol;
use crate::error::{SodsError, Result};
use std::time::{Instant, Duration};

const MAX_PATTERN_DEPTH: usize = 5;
const MAX_SYMBOLS_PER_PATTERN: usize = 10;
const PARSING_TIMEOUT_MS: u64 = 10;

#[derive(Debug, Clone, PartialEq)]
pub enum PatternCondition {
    None,
    FromDeployer,
    ValueGreaterThan(U256),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternStep {
    Exact(String, PatternCondition),
    AtLeast(String, usize, PatternCondition),
    Range(String, usize, usize, PatternCondition),
}

#[derive(Debug, Clone)]
pub struct BehavioralPattern {
    steps: Vec<PatternStep>,
}

impl BehavioralPattern {
    pub fn steps(&self) -> &[PatternStep] {
        &self.steps
    }

    /// Parse a pattern string into a BehavioralPattern.
    /// 
    /// Syntax:
    /// - "A -> B": Sequence of A then B
    /// - "A{n,}": At least n occurrences of A
    /// - "LP+ where from == deployer": Context filter
    /// - "Sandwich": Preset for "Tf -> Sw -> Tf"
    pub fn parse(input: &str) -> Result<Self> {
        let start_time = Instant::now();

        if input.len() > 500 {
            return Err(SodsError::PatternError("Pattern string too long (max 500 chars)".into()));
        }

        // 1. Check Presets
        match input {
            "Sandwich" => return Ok(Self {
                steps: vec![
                    PatternStep::Exact("Tf".into(), PatternCondition::None),
                    PatternStep::Exact("Sw".into(), PatternCondition::None),
                    PatternStep::Exact("Tf".into(), PatternCondition::None),
                ]
            }),
            "Frontrun" => return Ok(Self {
                steps: vec![
                    PatternStep::Exact("Tf".into(), PatternCondition::None),
                    PatternStep::Exact("Sw".into(), PatternCondition::None),
                ]
            }),
            "Backrun" => return Ok(Self {
                steps: vec![
                    PatternStep::Exact("Sw".into(), PatternCondition::None),
                    PatternStep::Exact("Tf".into(), PatternCondition::None),
                ]
            }),
            _ => {}
        }

        let parts: Vec<&str> = input.split("->").map(|s| s.trim()).collect();
        let mut steps = Vec::new();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            // Parse condition if present ("... where ...")
            let (part_base, condition) = if let Some(idx) = part.find("where") {
                let cond_str = part[idx+5..].trim();
                let base = part[..idx].trim();
                
                let cond = if cond_str == "from == deployer" {
                    PatternCondition::FromDeployer
                } else if let Some(stripped) = cond_str.strip_prefix("value >") {
                    let amount_str = stripped.trim();
                    let amount = parse_amount(amount_str)?;
                    PatternCondition::ValueGreaterThan(amount)
                } else {
                    return Err(SodsError::PatternError(format!("Unsupported condition: {}", cond_str)));
                };
                (base, cond)
            } else {
                (part, PatternCondition::None)
            };

            // Check for quantifier { ... }
            if let Some(start_idx) = part_base.find('{') {
                if let Some(end_idx) = part_base.find('}') {
                    let symbol = part_base[..start_idx].trim().to_string();
                    let quantifier = &part_base[start_idx+1..end_idx]; // inside {}

                    if let Some(comma_idx) = quantifier.find(',') {
                        let min_str = quantifier[..comma_idx].trim();
                        let max_str = quantifier[comma_idx+1..].trim();

                        let min = min_str.parse::<usize>().map_err(|_| SodsError::PatternError(format!("Invalid min quantifier: {}", min_str)))?;

                        if max_str.is_empty() {
                            // {n,}
                            steps.push(PatternStep::AtLeast(symbol, min, condition));
                        } else {
                            // {n,m}
                            let max = max_str.parse::<usize>().map_err(|_| SodsError::PatternError(format!("Invalid max quantifier: {}", max_str)))?;
                            steps.push(PatternStep::Range(symbol, min, max, condition));
                        }
                    } else {
                        // {n} exact count shorthand -> treat as Range(n, n)
                        let count = quantifier.trim().parse::<usize>().map_err(|_| SodsError::PatternError(format!("Invalid exact quantifier: {}", quantifier)))?;
                        steps.push(PatternStep::Range(symbol, count, count, condition));
                    }
                } else {
                    return Err(SodsError::PatternError(format!("Unmatched '{{' in pattern: {}", part_base)));
                }
            } else if part_base.contains('→') {
                 return Err(SodsError::PatternError(format!("Invalid character in symbol (did you mean '->'?): {}", part_base)));
            } else {
                // Single symbol
                steps.push(PatternStep::Exact(part_base.to_string(), condition));
            }

            // Check Limits
            if steps.len() > MAX_SYMBOLS_PER_PATTERN {
                return Err(SodsError::PatternError(format!("Pattern too complex (max {} symbols)", MAX_SYMBOLS_PER_PATTERN)));
            }

            if start_time.elapsed() > Duration::from_millis(PARSING_TIMEOUT_MS) {
                return Err(SodsError::PatternError("Pattern parsing timed out (DoS Protection)".into()));
            }
        }

        if steps.is_empty() {
             return Err(SodsError::PatternError("Empty pattern".to_string()));
        }

        Ok(Self { steps })
    }

    /// Check if the pattern matches the given sorted symbols.
    /// Returns the sequence of matched symbols if found, or None.
    pub fn matches<'a>(&self, symbols: &'a [BehavioralSymbol]) -> Option<Vec<&'a BehavioralSymbol>> {
        let mut matched_sequence = Vec::new();
        let mut current_sym_idx = 0;

        for step in &self.steps {
            if current_sym_idx >= symbols.len() {
                return None; // Ran out of symbols
            }

            match step {
                PatternStep::Exact(target, cond) => {
                    // Find first occurrence of target starting from current_sym_idx that satisfies condition
                    let found_idx = symbols[current_sym_idx..].iter().position(|s| {
                        s.symbol == *target && Self::check_condition(s, cond)
                    })?;
                    let absolute_idx = current_sym_idx + found_idx;
                    
                    matched_sequence.push(&symbols[absolute_idx]);
                    current_sym_idx = absolute_idx + 1;
                },
                PatternStep::AtLeast(target, min, cond) => {
                    let mut count = 0;
                    let mut temp_matched = Vec::new();
                    let mut idx = current_sym_idx;

                    while count < *min {
                         if idx >= symbols.len() {
                             return None; // Not enough symbols
                         }
                         
                         let sym = &symbols[idx];
                         if sym.symbol == *target && Self::check_condition(sym, cond) {
                             temp_matched.push(sym);
                             idx += 1;
                             count += 1;
                         } else {
                             return None; // Non-matching symbol in middle of quantifier sequence
                         }
                    }
                    matched_sequence.extend(temp_matched);
                    current_sym_idx = idx;
                },
                PatternStep::Range(target, min, _max, cond) => {
                    let mut count = 0;
                    let mut temp_matched = Vec::new();
                    let mut idx = current_sym_idx;

                    while count < *min {
                         if idx >= symbols.len() {
                             return None; 
                         }
                         
                         let sym = &symbols[idx];
                         if sym.symbol == *target && Self::check_condition(sym, cond) {
                             temp_matched.push(sym);
                             idx += 1;
                             count += 1;
                         } else {
                             return None; // Non-matching symbol in middle of range
                         }
                    }
                     matched_sequence.extend(temp_matched);
                     current_sym_idx = idx;
                }
            }
        }

        Some(matched_sequence)
    }

    fn check_condition(symbol: &BehavioralSymbol, condition: &PatternCondition) -> bool {
        match condition {
            PatternCondition::None => true,
            PatternCondition::FromDeployer => symbol.is_from_deployer,
            PatternCondition::ValueGreaterThan(threshold) => symbol.value > *threshold,
        }
    }
}

/// Helper function for ZK guest or simple matching
pub fn matches_str(symbols: &[BehavioralSymbol], pattern_str: &str) -> bool {
    if let Ok(p) = BehavioralPattern::parse(pattern_str) {
        p.matches(symbols).is_some()
    } else {
        false
    }
}

fn parse_amount(input: &str) -> Result<U256> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Err(SodsError::PatternError("Empty amount".to_string()));
    }

    if parts.len() == 1 {
        // Raw integer
        return U256::from_dec_str(parts[0])
            .map_err(|e| SodsError::PatternError(format!("Invalid amount: {}", e)));
    }

    if parts.len() == 2 {
        let value = parts[0].parse::<f64>()
            .map_err(|_| SodsError::PatternError(format!("Invalid number: {}", parts[0])))?;
        let unit = parts[1].to_lowercase();

        let multiplier = match unit.as_str() {
            "ether" => 1_000_000_000_000_000_000f64,
            "gwei" => 1_000_000_000f64,
            _ => return Err(SodsError::PatternError(format!("Unsupported unit: {}", unit))),
        };

        return Ok(U256::from((value * multiplier) as u128));
    }

    Err(SodsError::PatternError(format!("Malformed amount: {}", input)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_sym(s: &str, idx: u32) -> BehavioralSymbol {
        BehavioralSymbol::new(s, idx)
    }

    #[test]
    fn test_parse_simple() {
        let p = BehavioralPattern::parse("Tf -> Dep").unwrap();
        match &p.steps[0] {
            PatternStep::Exact(s, c) => { assert_eq!(s, "Tf"); assert_eq!(c, &PatternCondition::None); }
            _ => panic!("Wrong step type"),
        }
    }

    #[test]
    fn test_parse_condition() {
        let p = BehavioralPattern::parse("Tf where from == deployer -> Sw").unwrap();
        match &p.steps[0] {
            PatternStep::Exact(s, c) => { 
                assert_eq!(s, "Tf"); 
                assert_eq!(c, &PatternCondition::FromDeployer); 
            }
            _ => panic!("Wrong step type"),
        }
    }

    #[test]
    fn test_parse_preset() {
        let p = BehavioralPattern::parse("Sandwich").unwrap();
        assert_eq!(p.steps.len(), 3); // Tf -> Sw -> Tf
    }

    #[test]
    fn test_match_condition() {
        let mut sym1 = mock_sym("Tf", 0);
        sym1.is_from_deployer = true;
        let sym2 = mock_sym("Sw", 1);
        
        let symbols = vec![sym1, sym2.clone()];
        
        let p = BehavioralPattern::parse("Tf where from == deployer -> Sw").unwrap();
        assert!(p.matches(&symbols).is_some());

        // Test fail condition
        let sym3 = mock_sym("Tf", 0); // is_from_deployer = false by default
        let symbols_fail = vec![sym3, sym2];
         assert!(p.matches(&symbols_fail).is_none());
    }

    #[test]
    fn test_parse_frontrun_preset() {
        let p = BehavioralPattern::parse("Frontrun").unwrap();
        assert_eq!(p.steps.len(), 2);
        match &p.steps[0] {
            PatternStep::Exact(s, _) => assert_eq!(s, "Tf"),
            _ => panic!("Wrong step type"),
        }
        match &p.steps[1] {
            PatternStep::Exact(s, _) => assert_eq!(s, "Sw"),
            _ => panic!("Wrong step type"),
        }
    }

    #[test]
    fn test_parse_backrun_preset() {
        let p = BehavioralPattern::parse("Backrun").unwrap();
        assert_eq!(p.steps.len(), 2);
        match &p.steps[0] {
            PatternStep::Exact(s, _) => assert_eq!(s, "Sw"),
            _ => panic!("Wrong step type"),
        }
        match &p.steps[1] {
            PatternStep::Exact(s, _) => assert_eq!(s, "Tf"),
            _ => panic!("Wrong step type"),
        }
    }

    #[test]
    fn test_frontrun_match() {
        let symbols = vec![mock_sym("Tf", 0), mock_sym("Sw", 1)];
        let p = BehavioralPattern::parse("Frontrun").unwrap();
        assert!(p.matches(&symbols).is_some());
    }

    #[test]
    fn test_backrun_match() {
        let symbols = vec![mock_sym("Sw", 0), mock_sym("Tf", 1)];
        let p = BehavioralPattern::parse("Backrun").unwrap();
        assert!(p.matches(&symbols).is_some());
    }

    #[test]
    fn test_parse_value_condition() {
        let p = BehavioralPattern::parse("Tf where value > 1 ether").unwrap();
        match &p.steps[0] {
            PatternStep::Exact(s, PatternCondition::ValueGreaterThan(v)) => {
                assert_eq!(s, "Tf");
                assert_eq!(*v, U256::from(1_000_000_000_000_000_000u128));
            }
            _ => panic!("Wrong step type or condition"),
        }
    }

    #[test]
    fn test_match_value_condition() {
        use ethers_core::types::Address;
        let sym_high = BehavioralSymbol::new("Tf", 0)
            .with_context(Address::zero(), Address::zero(), U256::from(2_000_000_000_000_000_000u128), None);
        let sym_low = BehavioralSymbol::new("Tf", 1)
            .with_context(Address::zero(), Address::zero(), U256::from(500_000_000_000_000_000u128), None);
        
        let p = BehavioralPattern::parse("Tf where value > 1 ether").unwrap();
        
        // High value matches
        assert!(p.matches(&vec![sym_high]).is_some());
        // Low value fails
        assert!(p.matches(&vec![sym_low]).is_none());
    }

    #[test]
    fn test_parse_amount_units() {
        assert_eq!(parse_amount("10 ether").unwrap(), U256::from(10_000_000_000_000_000_000u128));
        assert_eq!(parse_amount("500 gwei").unwrap(), U256::from(500_000_000_000u128));
        assert_eq!(parse_amount("1000000").unwrap(), U256::from(1_000_000));
    }

    #[test]
    fn test_parse_invalid_amount() {
        assert!(parse_amount("10 eth").is_err()); // "ether" expected
        assert!(parse_amount("abc ether").is_err());
    }
    #[test]
    fn test_matches_str() {
        let symbols = vec![
            mock_sym("Tf", 0),
            mock_sym("Sw", 1),
            mock_sym("Tf", 2),
        ];
        assert!(matches_str(&symbols, "Tf -> Sw"));
        assert!(matches_str(&symbols, "Sandwich"));
        assert!(!matches_str(&symbols, "Dep"));
    }

    #[test]
    fn test_rigorous_quantifier_matching() {
        let symbols = vec![
            mock_sym("Sw", 0),
            mock_sym("Tf", 1),
            mock_sym("Sw", 2),
        ];
        
        // Exact {2} should FAIL because Tf is in the middle (it looks for EXACT SEQUENCE of Sw then Sw)
        // Wait, the current implementation of position(|s| s.symbol == *target) allows intermediate symbols.
        // Let's check the objective: "'Sw{2}' Valid: [Sw, Sw], Invalid: [Sw, Tf, Sw]"
        // My current 'matches' logic uses .iter().position(...) which finds the NEXT occurrence, 
        // essentially treating it as "contains sequence" rather than "exact adjacent sequence".
        
        // RE-AUDIT: If the objective requires [Sw, Tf, Sw] to FAIL for Sw{2}, I need to fix the logic.
        // The objective says "Invalid Sequence" for Sw{2} is [Sw, Tf, Sw].
        // So they MUST be adjacent.
        
        let p = BehavioralPattern::parse("Sw{2}").unwrap();
        assert!(p.matches(&vec![mock_sym("Sw", 0), mock_sym("Sw", 1)]).is_some());
        
        // This MUST FAIL now per objective audit
        assert!(p.matches(&symbols).is_none()); 
    }

    #[test]
    fn test_order_precision() {
        let symbols = vec![
            mock_sym("Sw", 0),
            mock_sym("LP+", 1),
        ];
        // LP+ -> Sw should fail because order is reversed
        assert!(matches_str(&symbols, "LP+ -> Sw") == false);
    }
}
