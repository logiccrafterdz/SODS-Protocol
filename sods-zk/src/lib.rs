use risc0_zkvm::{default_prover, ExecutorEnv, Receipt};
use sods_core::symbol::BehavioralSymbol;
use sods_zk_methods::{BehavioralProofInput, SODS_ZK_GUEST_ELF, SODS_ZK_GUEST_ID};
use anyhow::Result;

pub fn prove_behavior(
    symbols: Vec<BehavioralSymbol>,
    pattern: &str
) -> Result<Receipt> {
    // 1. Prepare input
    let input = BehavioralProofInput {
        symbols,
        pattern: pattern.to_string(),
    };

    // 2. Setup environment for the guest
    let env = ExecutorEnv::builder()
        .write(&input)?
        .build()?;

    // 3. Run the prover
    let prover = default_prover();
    let receipt = prover.prove(env, SODS_ZK_GUEST_ELF)?;

    // 4. Verify the receipt locally (optional but recommended)
    receipt.verify(SODS_ZK_GUEST_ID)?;

    Ok(receipt)
}

pub fn verify_receipt(receipt: &Receipt) -> Result<bool> {
    receipt.verify(SODS_ZK_GUEST_ID)?;
    let valid: bool = receipt.journal.decode()?;
    Ok(valid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sods_core::symbol::BehavioralSymbol;

    #[test]
    fn test_pattern_matching_logic() {
        let symbols = vec![
            BehavioralSymbol::new("Tf", 0),
            BehavioralSymbol::new("Sw", 1),
            BehavioralSymbol::new("Tf", 2),
        ];

        // This tests the underlying logic that the guest will run
        assert!(sods_core::pattern::matches_str(&symbols, "Tf -> Sw"));
        assert!(sods_core::pattern::matches_str(&symbols, "Sandwich"));
        assert!(!sods_core::pattern::matches_str(&symbols, "Dep"));
    }
}
