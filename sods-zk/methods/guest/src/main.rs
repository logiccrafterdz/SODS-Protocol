#![no_main]

use risc0_zkvm::guest::env;
use sods_zk_methods::BehavioralProofInput;

risc0_zkvm::guest::entry!(main);

fn main() {
    // Receive input from the host
    let input: BehavioralProofInput = env::read();
    
    // Run SODS core verification INSIDE the zkVM
    let is_valid = sods_core::pattern::matches_str(&input.symbols, &input.pattern);
    
    // Commit the result to the journal (public output)
    env::commit(&is_valid);
}
