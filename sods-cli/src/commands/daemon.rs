use clap::{Args, Subcommand};
use colored::Colorize;
use serde::Deserialize;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use sysinfo::{System, Pid};
#[cfg(unix)]
use dirs;
#[cfg(unix)]
use crate::commands::monitor::MonitorArgs; 

#[cfg(unix)]
use daemonize::Daemonize;
#[cfg(unix)]
use std::fs::File;

#[cfg(unix)]
use crate::config::get_chain;
use crate::output;
#[cfg(unix)]
use serde_json::json;

/// Arguments for the daemon command.
#[derive(Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommands,
}

#[derive(Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon in background
    Start {
        /// Behavioral pattern to monitor
        #[arg(short, long)]
        pattern: Option<String>,

        /// Blockchain chain
        #[arg(short, long, default_value = "sepolia")]
        chain: String,

        /// Polling interval
        #[arg(short, long, default_value = "30s")]
        interval: String,

        /// Custom RPC URL
        #[arg(long)]
        rpc_url: Option<String>,

        /// Generate auto-start script (systemd/launchd)
        #[arg(long)]
        autostart: bool,

        /// Forward alerts to a secure HTTPS webhook (e.g., ntfy.sh)
        #[arg(long)]
        webhook_url: Option<String>,
        
        /// Load behavioral threat patterns from a public feed (optional)
        #[arg(long)]
        threat_feed: Option<String>,
    },
    /// Stop the running daemon
    Stop,
    /// Check daemon status
    Status,
}

#[cfg(unix)]
#[derive(Deserialize, Debug, Clone)]
pub struct ThreatFeedItem {
    pub name: String,
    pub pattern: String,
    pub chain: String,
    pub severity: String,
    pub description: Option<String>,
}

#[cfg(unix)]
struct MonitoringTarget {
    pattern: sods_core::pattern::BehavioralPattern,
    name: String,
    severity: String,
    pattern_str: String,
    chain: String,
}

#[cfg(unix)]
fn get_sods_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Failed to get home directory");
    path.push(".sods");
    path
}

#[cfg(unix)]
fn get_pid_file() -> PathBuf {
    get_sods_dir().join("sods.pid")
}

#[cfg(unix)]
fn get_log_file() -> PathBuf {
    get_sods_dir().join("sods.log")
}

// -----------------------------------------------------------------------------
// Unix Implementation
// -----------------------------------------------------------------------------

#[cfg(unix)]
async fn start_daemon(
    pattern: Option<String>, 
    chain: String, 
    interval: String, 
    rpc_url: Option<String>, 
    autostart: bool, 
    webhook_url: Option<String>,
    threat_feed: Option<String>
) -> i32 {
    let sods_dir = get_sods_dir();
    if !sods_dir.exists() {
        fs::create_dir_all(&sods_dir).expect("Failed to create .sods dir");
    }

    let pid_file = get_pid_file();
    let log_file = get_log_file();

    // Check if running
    if check_status() {
        output::error("Daemon is already running.");
        return 1;
    }

    if autostart {
        println!("Genering systemd service file...");
        println!("Save the following to ~/.config/systemd/user/sods.service:");
        println!("[Unit]\nDescription=SODS Monitor\n[Service]\nExecStart={}", std::env::current_exe().unwrap().display());
        println!("Restart=always\n[Install]\nWantedBy=default.target");
        return 0;
    }

    // Fetch threat feed *before* daemonizing (to report errors clearly)
    let mut targets = Vec::new();
    
    // Add manual pattern if provided
    if let Some(p) = pattern {
        match sods_core::pattern::BehavioralPattern::parse(&p) {
            Ok(parsed) => {
                targets.push(MonitoringTarget {
                    pattern: parsed,
                    name: "Manual Pattern".to_string(),
                    severity: "manual".to_string(),
                    pattern_str: p,
                    chain: chain.clone(),
                });
            },
            Err(e) => {
                output::error(&format!("Invalid manual pattern: {}", e));
                return 1;
            }
        }
    }
    
    // Fetch threat feed
    if let Some(feed_url) = threat_feed {
        println!("Fetching threat feed from {}...", feed_url);
        match fetch_threat_feed(&feed_url).await {
            Ok(items) => {
                println!("Loaded {} patterns from threat feed.", items.len());
                for item in items {
                     if item.chain != chain {
                         continue; // Skip patterns for other chains
                     }
                     match sods_core::pattern::BehavioralPattern::parse(&item.pattern) {
                         Ok(parsed) => {
                             targets.push(MonitoringTarget {
                                 pattern: parsed,
                                 name: item.name,
                                 severity: item.severity,
                                 pattern_str: item.pattern,
                                 chain: item.chain,
                             });
                         },
                         Err(e) => eprintln!("⚠️ Skipping invalid pattern '{}': {}", item.name, e),
                     }
                }
            },
            Err(e) => {
                output::error(&format!("Failed to fetch threat feed: {}", e));
                if targets.is_empty() {
                    return 1; // Exit if no patterns at all
                }
            }
        }
    }
    
    if targets.is_empty() {
        output::error("No valid patterns to monitor. Provide --pattern or a valid --threat-feed.");
        return 1;
    }

    println!("Starting SODS daemon...");
    println!("Monitoring {} targets.", targets.len());
    println!("Logs: {}", log_file.display());
    println!("PID:  {}", pid_file.display());

    let stdout = std::fs::OpenOptions::new().create(true).append(true).open(&log_file).unwrap();
    let stderr = std::fs::OpenOptions::new().create(true).append(true).open(&log_file).unwrap();

    let daemonize = Daemonize::new()
        .pid_file(&pid_file)
        .chown_pid_file(true)
        .working_directory(&sods_dir)
        .stdout(stdout)
        .stderr(stderr);

    match daemonize.start() {
        Ok(_) => {
            // In daemon process
            run_daemon_loop(targets, chain, interval, rpc_url, webhook_url).await;
            0 
        }
        Err(e) => {
            eprintln!("Error, {}", e);
            1
        }
    }
}

#[cfg(unix)]
async fn fetch_threat_feed(url: &str) -> Result<Vec<ThreatFeedItem>, String> {
    if !url.starts_with("https://") {
        return Err("URL must be HTTPS".to_string());
    }
    
    let client = reqwest::Client::new();
    let res = client.get(url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| e.to_string())?;
        
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    
    let items: Vec<ThreatFeedItem> = res.json()
        .await
        .map_err(|e| format!("Invalid JSON: {}", e))?;
        
    Ok(items)
}

#[cfg(unix)]
async fn run_daemon_loop(
    targets: Vec<MonitoringTarget>, 
    chain: String, 
    interval_str: String, 
    rpc_url_opt: Option<String>,
    webhook_url: Option<String>
) {
    use sods_verifier::BlockVerifier;
    use crate::config::get_chain;
    use std::time::Duration;
    use tokio::time::sleep;
    use notify_rust::Notification;

    let interval_secs = if interval_str.ends_with("s") {
        interval_str.trim_end_matches("s").parse::<u64>().unwrap_or(30)
    } else { 30 };
    let interval = Duration::from_secs(interval_secs);
    
    let chain_config = get_chain(&chain).unwrap();
    let rpc_url = rpc_url_opt.as_deref().unwrap_or(chain_config.default_rpc);
    
    println!("Daemon loop started. Monitoring {} targets on {}", targets.len(), chain);
    if let Some(ref url) = webhook_url {
        println!("Webhook enabled: {}", url);
    }
    
    let verifier = BlockVerifier::new(rpc_url).unwrap();
    let mut last_scanned_block = verifier.get_latest_block().await.unwrap_or(0);

    loop {
        sleep(interval).await;
        
        match verifier.get_latest_block().await {
            Ok(current_head) => {
                if current_head > last_scanned_block {
                     for block_num in (last_scanned_block + 1)..=current_head {
                         // Fetch symbols ONCE per block
                         match verifier.fetch_block_symbols(block_num).await {
                             Ok(symbols) => {
                                 // Check all targets against these symbols
                                 for target in &targets {
                                     if target.pattern.matches(&symbols).is_some() {
                                         let msg = format!("🚨 {} ({}) detected on Block #{}", target.name, target.severity, block_num);
                                         println!("{}", msg);
                                         
                                         // Desktop Notification
                                         let _ = Notification::new()
                                             .summary("SODS Threat Alert 🚨")
                                             .body(&msg)
                                             .show();

                                         // Webhook
                                         if let Some(ref url) = webhook_url {
                                             let payload = json!({
                                                 "alert": "Behavioral pattern detected",
                                                 "chain": chain,
                                                 "block_number": block_num,
                                                 "pattern": target.pattern_str,
                                                 "threat_name": target.name,
                                                 "severity": target.severity,
                                                 "timestamp": chrono::Utc::now().to_rfc3339(),
                                                 "confidence": "High"
                                             });
                                             
                                             tokio::spawn(send_webhook(url.clone(), payload));
                                         }
                                     }
                                 }
                             },
                             Err(e) => eprintln!("Error fetching block #{}: {}", block_num, e),
                         }
                     }
                     last_scanned_block = current_head;
                }
            },
            Err(e) => println!("RPC Error: {}", e),
        }
    }
}

#[cfg(unix)]
async fn send_webhook(url: String, payload: serde_json::Value) {
    if !url.starts_with("https://") {
        return;
    }
    let client = reqwest::Client::new();
    let _ = client.post(&url)
        .json(&payload)
        .header("User-Agent", "SODS/1.2 (privacy-first behavioral monitor)")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
}


#[cfg(unix)]
fn stop_daemon() -> i32 {
    let pid_file = get_pid_file();
    if !pid_file.exists() {
        output::error("Daemon is not running (no PID file).");
        return 1;
    }

    let pid_str = fs::read_to_string(&pid_file).unwrap();
    let pid = pid_str.trim().parse::<i32>().unwrap();

    use std::process::Command;
    let _ = Command::new("kill").arg(pid.to_string()).output();
    
    let _ = fs::remove_file(pid_file);
    println!("Daemon stopped.");
    0
}

#[cfg(unix)]
fn check_status() -> bool {
    let pid_file = get_pid_file();
    if !pid_file.exists() {
        return false;
    }
    
    let pid_str = match fs::read_to_string(&pid_file) {
        Ok(s) => s,
        Err(_) => return false,
    };
    
    let pid_val = match pid_str.trim().parse::<usize>() {
         Ok(p) => p,
         Err(_) => return false,
    };

    let s = System::new_all();
    if let Some(_) = s.process(Pid::from(pid_val)) {
        true
    } else {
        false
    }
}

// -----------------------------------------------------------------------------
// Windows Implementation (Stub)
// -----------------------------------------------------------------------------

#[cfg(not(unix))]
async fn start_daemon(
    _pattern: Option<String>, 
    _chain: String, 
    _interval: String, 
    _rpc_url: Option<String>, 
    _autostart: bool, 
    _webhook_url: Option<String>,
    _threat_feed: Option<String>
) -> i32 {
    output::error("Daemon mode is currently only supported on Linux/macOS.");
    println!("👉 Use 'sods monitor' for foreground monitoring on Windows.");
    1
}

#[cfg(not(unix))]
fn stop_daemon() -> i32 {
    output::error("Daemon mode is not supported on Windows.");
    1
}

#[cfg(not(unix))]
fn check_status() -> bool {
    false
}


// -----------------------------------------------------------------------------
// Entry Point
// -----------------------------------------------------------------------------

pub async fn run(args: DaemonArgs) -> i32 {
    match args.command {
        DaemonCommands::Start { pattern, chain, interval, rpc_url, autostart, webhook_url, threat_feed } => {
            start_daemon(pattern, chain, interval, rpc_url, autostart, webhook_url, threat_feed).await
        },
        DaemonCommands::Stop => {
            stop_daemon()
        },
        DaemonCommands::Status => {
            if check_status() {
                println!("{}", "✅ SODS daemon is running".green().bold());
            } else {
                 println!("SODS daemon is not running.");
            }
            0
        }
    }
}
