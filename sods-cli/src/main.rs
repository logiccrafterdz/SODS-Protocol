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
#[command(about = "SODS Protocol - On-chain behavioral verification\nNote: Uses multiple RPC endpoints per chain for resilience.", long_about = None)]
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
    
    /// List supported behavioral symbols and load plugins
    Symbols(commands::symbols::SymbolsArgs),

    /// Discover behavioral hotspots in recent blocks
    Discover(commands::discover::DiscoverArgs),

    /// Detect behavioral trends
    Trend(commands::trend::TrendArgs),

    /// Autonomous behavioral monitoring
    Monitor(commands::monitor::MonitorArgs),

    /// System daemon management
    Daemon(commands::daemon::DaemonArgs),

    /// Manage decentralized threat intelligence
    Threats(commands::threats::ThreatsArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Verify(args) => commands::verify::run(args).await,
        Commands::Chains => commands::chains::run(),
        Commands::Symbols(args) => commands::symbols::run(args).await,
        Commands::Discover(args) => commands::discover::run(args).await,
        Commands::Trend(args) => commands::trend::run(args).await,
        Commands::Monitor(args) => commands::monitor::run(args).await,
        Commands::Daemon(args) => commands::daemon::run(args).await,
        Commands::Threats(args) => commands::threats::run(args).await,
    };

    std::process::exit(exit_code);
}
