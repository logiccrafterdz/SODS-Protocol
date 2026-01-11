//! SODS Protocol CLI
//!
//! Terminal-first interface for on-chain behavioral verification.

mod commands;
mod config;
mod output;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sods")]
#[command(author = "LogicCrafter")]
#[command(version = "0.1.0")]
#[command(about = "SODS Protocol - On-chain behavioral verification", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a behavioral symbol in a block
    Verify(commands::verify::VerifyArgs),
    
    /// List supported blockchain chains
    Chains,
    
    /// List supported behavioral symbols
    Symbols,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Verify(args) => commands::verify::run(args).await,
        Commands::Chains => commands::chains::run(),
        Commands::Symbols => commands::symbols::run(),
    };

    std::process::exit(exit_code);
}
