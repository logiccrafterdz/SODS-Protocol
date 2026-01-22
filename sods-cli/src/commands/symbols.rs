//! Symbols command implementation.

use clap::{Args, Subcommand};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::config::SYMBOLS;
use crate::output;
use sods_core::plugins::SymbolPlugin;

#[derive(Args)]
pub struct SymbolsArgs {
    #[command(subcommand)]
    pub command: Option<SymbolsCmd>,
}

#[derive(Subcommand)]
pub enum SymbolsCmd {
    /// List supported symbols (default)
    List,
    /// Load a symbol plugin from a URL (JSON)
    Load {
        url: String,
    },
}

/// Run the symbols command.
pub async fn run(args: SymbolsArgs) -> i32 {
    match args.command.unwrap_or(SymbolsCmd::List) {
        SymbolsCmd::List => list_symbols(),
        SymbolsCmd::Load { url } => load_symbol_plugin(&url).await,
    }
}

fn list_symbols() -> i32 {
    output::header("Supported Behavioral Symbols");
    
    println!();
    println!(
        "{:<8} {}",
        "Symbol".bold(),
        "Meaning".bold()
    );
    println!("{}", "─".repeat(40).dimmed());
    
    // Built-ins
    for (symbol, meaning) in SYMBOLS {
        println!(
            "{:<8} {}",
            symbol.green().bold(),
            meaning
        );
    }

    // Loaded Plugins
    if let Ok(plugins) = load_local_plugins() {
        if !plugins.is_empty() {
             println!("{}", "─".repeat(40).dimmed());
             for p in plugins {
                 println!(
                    "{:<8} {} (Plugin)",
                    p.symbol.green().bold(),
                    p.name
                );
             }
        }
    }
    
    println!();
    println!();
    output::info("Estos símbolos representan eventos de comportamiento en cadena.");
    output::hint("You can verify single symbols: sods verify <SYMBOL> --block <BLOCK>");
    
    // ... existing DSL hints ...
    println!();
    output::header("Behavioral Patterns DSL");
    output::info("You can also verify complex behavioral sequences:");
    println!("  {:<20} {}", "Sequence", "Use '->' (e.g., 'LP+ -> Sw -> LP-')");
    println!("  {:<20} {}", "Exact Count", "Use '{n}' (e.g., 'Sw{3}' for 3 Swaps)");
    
    println!();
    output::hint("Load new symbols: sods symbols load <URL>");
    
    0
}

async fn load_symbol_plugin(url: &str) -> i32 {
    output::header("Loading Symbol Plugin...");
    println!("   Source: {}", url);

    // Fetch
    let resp = match reqwest::get(url).await {
        Ok(r) => r,
        Err(e) => {
            output::error(&format!("Failed to fetch plugin: {}", e));
            return 1;
        }
    };

    let body = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            output::error(&format!("Failed to read body: {}", e));
            return 1;
        }
    };

    // Validate
    let plugin = match SymbolPlugin::load_from_json(&body) {
        Ok(p) => p,
        Err(e) => {
            output::error(&format!("Invalid plugin JSON: {}", e));
            return 1;
        }
    };

    println!("   Plugin: {} ({})", plugin.name.cyan(), plugin.symbol);
    println!("   Topic:  {:?}", plugin.event_topic);

    // Save
    if let Err(e) = save_plugin(&plugin, &body) {
        output::error(&format!("Failed to save plugin: {}", e));
        return 1;
    }

    output::success("Plugin loaded successfully!");
    0
}

fn get_plugins_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let dir = home.join(".sods").join("plugins");
    fs::create_dir_all(&dir).ok();
    dir
}

fn save_plugin(plugin: &SymbolPlugin, json_content: &str) -> std::io::Result<()> {
    let dir = get_plugins_dir();
    let filename = format!("{}.json", plugin.symbol);
    fs::write(dir.join(filename), json_content)
}

pub fn load_local_plugins() -> std::io::Result<Vec<SymbolPlugin>> {
    let dir = get_plugins_dir();
    let mut plugins = Vec::new();
    
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)?;
                if let Ok(p) = SymbolPlugin::load_from_json(&content) {
                    plugins.push(p);
                }
            }
        }
    }
    Ok(plugins)
}
