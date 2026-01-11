//! Chains command implementation.

use colored::Colorize;

use crate::config::CHAINS;
use crate::output;

/// Run the chains command.
pub fn run() -> i32 {
    output::header("Supported Chains");
    
    println!();
    println!(
        "{:<12} {:<12} {}",
        "Name".bold(),
        "Chain ID".bold(),
        "Description".bold()
    );
    println!("{}", "─".repeat(50).dimmed());
    
    for chain in CHAINS {
        println!(
            "{:<12} {:<12} {}",
            chain.name.green(),
            chain.chain_id,
            chain.description.dimmed()
        );
    }
    
    println!();
    output::hint("Use --chain <NAME> with verify command.");
    output::hint("Use --rpc-url to override default RPC.");
    
    0
}
