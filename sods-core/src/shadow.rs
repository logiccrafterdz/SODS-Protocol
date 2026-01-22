use ethers_core::types::Address;
use crate::symbol::BehavioralSymbol;
use crate::pattern::{BehavioralPattern, PatternStep, PatternCondition};

#[derive(Debug, Clone, PartialEq)]
pub enum ShadowStatus {
    Active,
    Resolved,
    Deviation(String), // Reason
    Expired,
}

/// A predictive shadow that tracks an actor's behavior against an expected pattern.
#[derive(Debug, Clone)]
pub struct BehavioralShadow {
    pub actor: Address,
    pub start_nonce: u64,
    pub steps: Vec<PatternStep>,
    pub current_step_index: usize,
    pub timeout_block: u64,
    pub created_at_block: u64,
}

impl BehavioralShadow {
    pub fn new(
        actor: Address, 
        start_nonce: u64, 
        steps: Vec<PatternStep>, 
        created_at_block: u64
    ) -> Self {
        Self {
            actor,
            start_nonce,
            steps,
            current_step_index: 0, // 0 is usually the start trigger
            timeout_block: created_at_block + 10, // Default 10 block timeout
            created_at_block,
        }
    }

    /// Process new symbols for a block and update state.
    pub fn check_block(&mut self, current_block: u64, symbols: &[BehavioralSymbol]) -> ShadowStatus {
        if current_block > self.timeout_block {
            return ShadowStatus::Expired;
        }

        // Filter symbols for this actor
        let actor_symbols: Vec<&BehavioralSymbol> = symbols.iter()
            .filter(|s| s.from == self.actor)
            .collect();

        if actor_symbols.is_empty() {
            return ShadowStatus::Active; // Waiting
        }

        // Process each symbol from actor to advance state
        for sym in actor_symbols {
            // Check causality: Nonce must be strictly increasing from start?
            // Or just >= start_nonce? A shadow tracks a specific flow.
            // If nonce < start_nonce, ignore (old txs reorged? or out of order)
            if sym.nonce < self.start_nonce {
                continue; 
            }
            
            // Current expected step
            if self.current_step_index >= self.steps.len() {
                return ShadowStatus::Resolved;
            }

            let expected_step = &self.steps[self.current_step_index];
            
            match expected_step {
                PatternStep::Exact(target_res, cond) => {
                    if sym.symbol == *target_res {
                         // Check condition
                        if Self::check_condition(sym, cond) {
                            self.current_step_index += 1;
                        } else {
                            // Symbol matches but condition fails? 
                            // e.g. "Tf" but not "from deployer" (if checking condition on every step)
                            // Strict partial matching: If valid symbol but invalid condition => Deviation?
                            // Or just ignore? 
                            // "Predictive" means we expect specific behavior. 
                            // If they do "Tf (normal)" instead of "Tf (deployer)", is it a deviation?
                            // For safety, let's say strict matching.
                             return ShadowStatus::Deviation(format!("Condition check failed for {} at step {}", sym.symbol, self.current_step_index));
                        }
                    } else {
                        // Unexpected symbol from actor!
                        // If I expect "Sw" but see "BridgeOut", that is a deviation.
                        return ShadowStatus::Deviation(format!("Unexpected symbol: Expected {}, got {}", target_res, sym.symbol));
                    }
                },
                _ => {
                    // Logic for Range/AtLeast is complex for shadowing (stateful counting).
                    // For MVP V1.1, we assume expanded Exact steps or simple matching.
                    // If complex step, just skip or auto-resolve for now to avoid complexity explosion in MVP.
                    self.current_step_index += 1; 
                }
            }
        }
        
        if self.current_step_index >= self.steps.len() {
            ShadowStatus::Resolved
        } else {
            ShadowStatus::Active
        }
    }
    
    fn check_condition(symbol: &BehavioralSymbol, condition: &PatternCondition) -> bool {
        match condition {
            PatternCondition::None => true,
            PatternCondition::FromDeployer => symbol.is_from_deployer,
        }
    }
    
    /// Extract steps from a pattern to initialize shadow.
    pub fn from_pattern(pattern: &BehavioralPattern, actor: Address, nonce: u64, block: u64) -> Self {
         Self::new(actor, nonce, pattern.steps().to_vec(), block) 
    }
}
