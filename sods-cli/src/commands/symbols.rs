//! Symbols command implementation.

use colored::Colorize;

use crate::config::SYMBOLS;
use crate::output;

/// Run the symbols command.
pub fn run() -> i32 {
    output::header("Supported Behavioral Symbols");
    
    println!();
    println!(
        "{:<8} {}",
        "Symbol".bold(),
        "Meaning".bold()
    );
    println!("{}", "─".repeat(40).dimmed());
    
    for (symbol, meaning) in SYMBOLS {
        println!(
            "{:<8} {}",
            symbol.green().bold(),
            meaning
        );
    }
    
    println!();
    output::info("These symbols represent on-chain behavioral events.");
    output::hint("Use with: sods verify <SYMBOL> --block <BLOCK>");
    
    0
}
