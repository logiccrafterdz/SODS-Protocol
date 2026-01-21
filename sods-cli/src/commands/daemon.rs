use clap::{Args, Subcommand};
use colored::Colorize;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use sysinfo::{System, Pid};
#[cfg(unix)]
use dirs; // dirs is mainly used for home dir path on unix for PID file
#[cfg(unix)]
use crate::commands::monitor::{self, MonitorArgs}; 

#[cfg(unix)]
use daemonize::Daemonize;
#[cfg(unix)]
use std::fs::File;

use crate::config::get_chain;
use crate::output;

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
        pattern: String,

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
    },
    /// Stop the running daemon
    Stop,
    /// Check daemon status
    Status,
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
async fn start_daemon(pattern: String, chain: String, interval: String, rpc_url: Option<String>, autostart: bool) -> i32 {
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
        // TODO: Implement actual generation logic or print helpful instructions
        println!("Save the following to ~/.config/systemd/user/sods.service:");
        println!("[Unit]\nDescription=SODS Monitor\n[Service]\nExecStart={}", std::env::current_exe().unwrap().display());
        println!("Restart=always\n[Install]\nWantedBy=default.target");
        return 0;
    }

    println!("Starting SODS daemon...");
    println!("Logs: {}", log_file.display());
    println!("PID:  {}", pid_file.display());

    let stdout = File::create(&log_file).unwrap();
    let stderr = File::create(&log_file).unwrap(); // Same log file for now or separate? Appending?
    // Proper logging usually requires OpenOptions::append()
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
            // We are now in the daemon process
            let args = MonitorArgs {
                pattern,
                chain,
                interval,
                rpc_url,
            };
            
            // Run the monitor logic
            // Note: monitor::run prints to stdout/stderr, which are now redirected to log file.
            // We need to inject notification logic. monitor::run doesn't support generic callback yet.
            // For MVP, we'll just run monitor::run. The notification part requires modifying monitor.rs 
            // OR reimplementing the loop here. 
            // Reusing is better. Let's assume monitor::run is enough for "logging alerts". 
            // DESKTOP NOTIFICATION is a key requirement.
            // We can modify monitor.rs to emit a notification if a feature flag is on, or simply copy the loop logic here since it's short.
            // Copying is safer to modify output behavior (we don't want ANSI colors in log file ideally, but it's ok).
            
            // Re-implementing simplified loop with notification:
            run_daemon_loop(args).await;
            0 
        }
        Err(e) => {
            eprintln!("Error, {}", e);
            1
        }
    }
}

#[cfg(unix)]
async fn run_daemon_loop(args: MonitorArgs) {
    use sods_core::pattern::BehavioralPattern;
    use sods_verifier::BlockVerifier;
    use crate::config::get_chain;
    use std::time::Duration;
    use tokio::time::sleep;
    use notify_rust::Notification;

    // ... (Simplified parsing logic from monitor.rs) ...
    // Assuming args are valid since checked before daemonizing? No, need to re-parse.
    
    let interval_secs = if args.interval.ends_with("s") {
        args.interval.trim_end_matches("s").parse::<u64>().unwrap_or(30)
    } else { 30 };
    let interval = Duration::from_secs(interval_secs);
    
    let pattern = BehavioralPattern::parse(&args.pattern).unwrap();
    let chain_config = get_chain(&args.chain).unwrap();
    let rpc_url = args.rpc_url.as_deref().unwrap_or(chain_config.default_rpc);
    
    println!("Daemon started. Monitoring {} on {}", args.pattern, args.chain);
    
    let verifier = BlockVerifier::new(rpc_url).unwrap();
    let mut last_scanned_block = verifier.get_latest_block().await.unwrap_or(0);

    loop {
        sleep(interval).await;
        
        match verifier.get_latest_block().await {
            Ok(current_head) => {
                if current_head > last_scanned_block {
                     for block_num in (last_scanned_block + 1)..=current_head {
                         if let Ok(symbols) = verifier.fetch_block_symbols(block_num).await {
                             if pattern.matches(&symbols).is_some() {
                                 let msg = format!("Pattern {} detected on Block #{}", args.pattern, block_num);
                                 println!("{}", msg);
                                 
                                 // Desktop Notification
                                 let _ = Notification::new()
                                     .summary("SODS Alert 🚨")
                                     .body(&msg)
                                     .show();
                             }
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
fn stop_daemon() -> i32 {
    let pid_file = get_pid_file();
    if !pid_file.exists() {
        output::error("Daemon is not running (no PID file).");
        return 1;
    }

    let pid_str = fs::read_to_string(&pid_file).unwrap();
    let pid = pid_str.trim().parse::<i32>().unwrap(); // sysinfo uses Pid, which wraps usize/i32 depending on platform

    // kill process
    // Using libc or Command kill
    use std::process::Command;
    let _ = Command::new("kill").arg(pid.to_string()).output();
    
    // Clean up
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
    
    let pid_val = match pid_str.trim().parse::<usize>() { // Sysinfo uses usize for Pid on some platforms
         Ok(p) => p,
         Err(_) => return false,
    };

    let s = System::new_all();
    if let Some(_) = s.process(Pid::from(pid_val)) {
        true
    } else {
        // Stale pid file
        false
    }
}

// -----------------------------------------------------------------------------
// Windows Implementation (Stub)
// -----------------------------------------------------------------------------

#[cfg(not(unix))]
async fn start_daemon(_pattern: String, _chain: String, _interval: String, _rpc_url: Option<String>, _autostart: bool) -> i32 {
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
        DaemonCommands::Start { pattern, chain, interval, rpc_url, autostart } => {
            start_daemon(pattern, chain, interval, rpc_url, autostart).await
        },
        DaemonCommands::Stop => {
            stop_daemon()
        },
        DaemonCommands::Status => {
            if check_status() {
                println!("{}", "✅ SODS daemon is running".green().bold());
                // Ideally read config from somewhere to say WHAT is running
            } else {
                 println!("SODS daemon is not running.");
            }
            0
        }
    }
}
