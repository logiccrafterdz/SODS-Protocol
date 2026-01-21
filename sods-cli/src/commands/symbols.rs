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
    println!();
    output::info("These symbols represent on-chain behavioral events.");
    output::hint("You can verify single symbols: sods verify <SYMBOL> --block <BLOCK>");
    
    println!();
    output::header("Behavioral Patterns DSL");
    output::info("You can also verify complex behavioral sequences:");
    println!("  {:<20} {}", "Sequence", "Use '->' (e.g., 'LP+ -> Sw -> LP-')");
    println!("  {:<20} {}", "Exact Count", "Use '{n}' (e.g., 'Sw{3}' for 3 Swaps)");
    println!("  {:<20} {}", "At Least", "Use '{n,}' (e.g., 'Tf{2,}' for 2+ Transfers)");
    println!("  {:<20} {}", "Range", "Use '{n,m}' (e.g., 'Appr{1,3}' for 1-3 Approvals)");
    
    println!();
    output::hint("Example: sods verify 'LP+ -> Sw{2,} -> LP-' --block <BLOCK>");
    
    0
}
