//! Threat Intelligence management commands.

use clap::{Args, Subcommand};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;
use sods_p2p::ThreatRule;

#[derive(Args)]
pub struct ThreatsArgs {
    #[command(subcommand)]
    pub command: ThreatsCommands,
}

#[derive(Subcommand)]
pub enum ThreatsCommands {
    /// List active threat rules
    List,
    /// Add a trusted researcher public key (hex)
    AddKey { 
        /// Compressed public key (33 bytes) in hex
        key: String 
    },
}

pub async fn run(args: ThreatsArgs) -> i32 {
    match args.command {
        ThreatsCommands::List => list_rules(),
        ThreatsCommands::AddKey { key } => add_key(key),
    }
}

fn get_sods_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Home directory not found");
    path.push(".sods");
    fs::create_dir_all(&path).ok();
    path
}

fn list_rules() -> i32 {
    let mut path = get_sods_dir();
    path.push("threat_rules.json");

    if !path.exists() {
        println!("No active threat rules found.");
        return 0;
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read threat registry: {}", e);
            return 1;
        }
    };

    let rules: Vec<ThreatRule> = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to parse threat registry: {}", e);
            return 1;
        }
    };

    println!("{}", "Active P2P Threat Rules:".green().bold());
    for rule in rules {
        println!("- {} (severity: {})", rule.id.cyan(), rule.severity);
        println!("  Pattern: {}", rule.pattern);
        println!("  Chain:   {}", rule.chain);
        println!();
    }

    0
}

fn add_key(key_hex: String) -> i32 {
    let key_bytes = match hex::decode(&key_hex) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("Invalid hex string");
            return 1;
        }
    };

    if key_bytes.len() != 33 {
        eprintln!("Invalid key length (expected 33 bytes compressed)");
        return 1;
    }

    let mut path = get_sods_dir();
    path.push("trusted_keys.json");

    let mut keys: Vec<String> = if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "[]".to_string());
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    if !keys.contains(&key_hex) {
        keys.push(key_hex.clone());
        let json = serde_json::to_string_pretty(&keys).unwrap();
        if let Err(e) = fs::write(path, json) {
            eprintln!("Failed to save key: {}", e);
            return 1;
        }
        println!("Added trusted key: {}", key_hex.green());
    } else {
        println!("Key already trusted.");
    }

    0
}
