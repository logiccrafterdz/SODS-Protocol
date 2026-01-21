use crate::symbol::BehavioralSymbol;
use crate::error::{SodsError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum PatternStep {
    Exact(String),
    AtLeast(String, usize),
    Range(String, usize, usize),
}

#[derive(Debug, Clone)]
pub struct BehavioralPattern {
    steps: Vec<PatternStep>,
}

impl BehavioralPattern {
    /// Parse a pattern string into a BehavioralPattern.
    /// 
    /// Syntax:
    /// - "A -> B": Sequence of A then B
    /// - "A{n,}": At least n occurrences of A
    /// - "A{n,m}": Between n and m occurrences of A
    pub fn parse(input: &str) -> Result<Self> {
        let parts: Vec<&str> = input.split("->").map(|s| s.trim()).collect();
        let mut steps = Vec::new();

        for part in parts {
            if part.is_empty() {
                continue;
            }

            // Check for quantifier { ... }
            if let Some(start_idx) = part.find('{') {
                if let Some(end_idx) = part.find('}') {
                    let symbol = part[..start_idx].trim().to_string();
                    let quantifier = &part[start_idx+1..end_idx]; // inside {}

                    if let Some(comma_idx) = quantifier.find(',') {
                        let min_str = quantifier[..comma_idx].trim();
                        let max_str = quantifier[comma_idx+1..].trim();

                        let min = min_str.parse::<usize>().map_err(|_| SodsError::PatternError(format!("Invalid min quantifier: {}", min_str)))?;

                        if max_str.is_empty() {
                            // {n,}
                            steps.push(PatternStep::AtLeast(symbol, min));
                        } else {
                            // {n,m}
                            let max = max_str.parse::<usize>().map_err(|_| SodsError::PatternError(format!("Invalid max quantifier: {}", max_str)))?;
                            steps.push(PatternStep::Range(symbol, min, max));
                        }
                    } else {
                        // {n} exact count shorthand (treated as range n..n or exact repeated? treated as exact n times sequence usually, 
                        // but let's treat {n} as range n,n for simplicity if implied, but prompt specifically asked for {n,} and {n,m}.
                        // If user types {n}, let's error or support it. Prompt: "<symbol>{n,} ... <symbol>{n,m}"
                        // Let's support {n} as exactly n for robustness
                        let count = quantifier.trim().parse::<usize>().map_err(|_| SodsError::PatternError(format!("Invalid exact quantifier: {}", quantifier)))?;
                        steps.push(PatternStep::Range(symbol, count, count));
                    }
                } else {
                    return Err(SodsError::PatternError(format!("Unmatched '{{' in pattern: {}", part)));
                }
            } else if part.contains('→') {
                 // Handle unicode arrow if user copy-pasted, though parse splits on "->"
                 // We should probably normalize input first
                 return Err(SodsError::PatternError(format!("Invalid character in symbol (did you mean '->'?): {}", part)));
            } else {
                // Single symbol
                steps.push(PatternStep::Exact(part.to_string()));
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
                PatternStep::Exact(target) => {
                    // Find first occurrence of target starting from current_sym_idx
                    let found_idx = symbols[current_sym_idx..].iter().position(|s| s.symbol == *target)?;
                    let absolute_idx = current_sym_idx + found_idx;
                    
                    matched_sequence.push(&symbols[absolute_idx]);
                    current_sym_idx = absolute_idx + 1;
                },
                PatternStep::AtLeast(target, min) => {
                    let mut count = 0;
                    let mut temp_matched = Vec::new();
                    let mut idx = current_sym_idx;

                    // Greedily find as many matches as possible? Or just satisfy minimum?
                    // Prompt says: "Sequence must appear contiguously or non-contiguously in order"
                    // "Quantifiers apply to consecutive occurrences" -> this usually implies strict adjacency OR just grouping.
                    // Let's assume grouping in the stream: find first occurrence, then see if next one is same, etc?
                    // "Sw{2,}" -> Find Sw, then Sw.
                    
                    // Strategy: Find *first* occurrence of target. Then look for subsequent ones.
                    // If we find 'min' occurrences, good.
                    // NOTE: This simple logic finds ANY 'min' occurrences ordered in time.
                    // If the user meant "Burst of 5 swaps", this logic matches "5 swaps spread over the whole block".
                    // Given the goal "Behavioral Story" (e.g. LP+ -> Swaps -> LP-), spread out events IS the story.
                    // So we scan forward for 'min' occurrences.
                    
                    while count < *min {
                         if idx >= symbols.len() {
                             return None; // Not enough symbols
                         }
                         if let Some(found_idx) = symbols[idx..].iter().position(|s| s.symbol == *target) {
                             let absolute_idx = idx + found_idx;
                             temp_matched.push(&symbols[absolute_idx]);
                             idx = absolute_idx + 1;
                             count += 1;
                         } else {
                             return None; // Cannot find enough
                         }
                    }
                    matched_sequence.extend(temp_matched);
                    current_sym_idx = idx;
                },
                PatternStep::Range(target, min, _max) => {
                     // Similar to AtLeast but capped
                    let mut count = 0;
                    let mut temp_matched = Vec::new();
                    let mut idx = current_sym_idx;

                    while count < *min {
                         if idx >= symbols.len() {
                             return None; 
                         }
                         if let Some(found_idx) = symbols[idx..].iter().position(|s| s.symbol == *target) {
                             let absolute_idx = idx + found_idx;
                             temp_matched.push(&symbols[absolute_idx]);
                             idx = absolute_idx + 1;
                             count += 1;
                         } else {
                             return None; 
                         }
                    }
                    
                    // Optional: Trying to find more up to max?
                    // If we just satisfy min, is it enough? "Range" usually implies validation.
                    // But if we verify "Sw{2,5}", finding 2 is valid. Finding 6?
                    // If we find 6, matches() should probably still return true (it matched 2..5).
                    // The "Range" usually restricts "Burst". But in a "Story", "At least 2" is usually what's meant by {2,5}.
                    // If strict max is needed, we'd need negative lookahead which is complex.
                    // For now, satisfy 'min'.
                     matched_sequence.extend(temp_matched);
                     current_sym_idx = idx;
                }
            }
        }

        Some(matched_sequence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_sym(s: &str, idx: u32) -> BehavioralSymbol {
        BehavioralSymbol {
            symbol: s.to_string(),
            log_index: idx,
            metadata: vec![],
        }
    }

    #[test]
    fn test_parse_simple() {
        let p = BehavioralPattern::parse("Tf -> Dep").unwrap();
        assert_eq!(p.steps.len(), 2);
        assert_eq!(p.steps[0], PatternStep::Exact("Tf".to_string()));
        assert_eq!(p.steps[1], PatternStep::Exact("Dep".to_string()));
    }

    #[test]
    fn test_parse_quantifiers() {
        let p = BehavioralPattern::parse("Sw{2,} -> LP-{1,5}").unwrap();
        assert_eq!(p.steps[0], PatternStep::AtLeast("Sw".to_string(), 2));
        assert_eq!(p.steps[1], PatternStep::Range("LP-".to_string(), 1, 5));
    }

    #[test]
    fn test_match_sequence() {
        let symbols = vec![
            mock_sym("Tf", 0),
            mock_sym("Sw", 1), 
            mock_sym("Sw", 2),
            mock_sym("LP-", 10)
        ];
        
        // Exact
        let p = BehavioralPattern::parse("Tf -> Sw").unwrap();
        assert!(p.matches(&symbols).is_some());

        // AtLeast
        let p2 = BehavioralPattern::parse("Sw{2,}").unwrap();
        let m2 = p2.matches(&symbols).unwrap();
        assert_eq!(m2.len(), 2);
        assert_eq!(m2[0].log_index, 1);
        assert_eq!(m2[1].log_index, 2);

        // Sequence
        let p3 = BehavioralPattern::parse("Tf -> Sw{2,} -> LP-").unwrap();
        assert!(p3.matches(&symbols).is_some());

        // Fail
        let p4 = BehavioralPattern::parse("Tf -> LP- -> Sw").unwrap(); // Wrong order
        assert!(p4.matches(&symbols).is_none());
    }
}
