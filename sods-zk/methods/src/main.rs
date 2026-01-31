use risc0_zkvm::guest::env;
use sods_core::symbol::BehavioralProofInput;

risc0_zkvm::guest::entry!(main);

fn main() {
    // Receive input from the host
    let input: BehavioralProofInput = env::read();
    
    // Run SODS core verification INSIDE the zkVM
    let is_valid = sods_core::pattern::matches_str(&input.symbols, &input.pattern);
    
    // Commit rich metadata to the journal for on-chain verification
    // Tuple: (blockNumber, chainId, pattern, result)
    env::commit(&(input.block_number, input.chain_id, input.pattern, is_valid));
}
