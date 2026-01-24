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
    /// Verify behavioral patterns using Behavioral Merkle Trees (BMT). Causal Merkle Trees (CMT) coming in v8.0.
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

    /// Export an on-chain verifiable behavioral proof
    ExportProof(commands::export_proof::ExportProofArgs),

    /// Compute the privacy-safe hash of a behavioral pattern
    HashPattern(commands::hash_pattern::HashPatternArgs),

    /// Generate a Zero-Knowledge proof of behavior
    ZkProve(commands::zk_prove::ZkProveArgs),
    
    /// Manage the contract deployer registry
    Registry(commands::registry::RegistryArgs),
}

fn main() {
    let cli = Cli::parse();

    // Special handling for Daemon to avoid fork issues with tokio threads
    if let Commands::Daemon(args) = cli.command {
        let exit_code = commands::daemon::run_sync(args);
        std::process::exit(exit_code);
    }

    // Standard async runtime for other commands
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    let exit_code = rt.block_on(async {
        match cli.command {
            Commands::Verify(args) => commands::verify::run(args).await,
            Commands::Chains => commands::chains::run(),
            Commands::Symbols(args) => commands::symbols::run(args).await,
            Commands::Discover(args) => commands::discover::run(args).await,
            Commands::Trend(args) => commands::trend::run(args).await,
            Commands::Monitor(args) => commands::monitor::run(args).await,
            Commands::Threats(args) => commands::threats::run(args).await,
            Commands::ExportProof(args) => commands::export_proof::run(args).await,
            Commands::HashPattern(args) => commands::hash_pattern::run(args).await,
            Commands::ZkProve(args) => commands::zk_prove::run(args).await,
            Commands::Registry(args) => commands::registry::run(args),
            Commands::Daemon(_) => unreachable!(), // Handled above
        }
    });

    std::process::exit(exit_code);
}
