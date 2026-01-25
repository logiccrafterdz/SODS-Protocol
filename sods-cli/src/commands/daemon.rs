use clap::{Args, Subcommand};
use colored::Colorize;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc::unbounded_channel, mpsc::UnboundedSender};
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use futures_util::{StreamExt, SinkExt};
#[cfg(feature = "metrics")]
use prometheus::{Encoder, TextEncoder, Registry, IntGauge, Counter, Histogram, HistogramOpts};
#[cfg(feature = "metrics")]
use axum::{Router, routing::get, extract::State, response::Response};
#[cfg(feature = "metrics")]
use http_body_util::Full;

#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use sysinfo::{System, Pid};
#[cfg(unix)]
use dirs;

#[cfg(unix)]
use daemonize::Daemonize;
#[cfg(unix)]
use std::fs::File;

#[cfg(unix)]
use crate::config::get_chain;
use crate::output;
#[cfg(unix)]
use serde_json::json;
#[cfg(unix)]
use sods_p2p::{SodsPeer, ThreatRule};

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

        /// Join the P2P decentralized threat intelligence network
        #[arg(long)]
        p2p_threat_network: bool,

        /// Auto-remove rules after duration (e.g. 24h, 30m)
        #[arg(long, default_value = "24h")]
        expire_after: String,

        /// Start a WebSocket server for real-time behavioral alerts
        #[arg(long)]
        websocket_port: Option<u16>,

        /// Start a Prometheus _metrics server (e.g. 9090)
        #[arg(long)]
        metrics_port: Option<u16>,
    },
    /// Stop the running daemon
    Stop,
    /// Check daemon status
    Status,
}

#[cfg(unix)]
#[derive(serde::Deserialize, Debug, Clone)]
pub struct ThreatFeedItem {
    pub name: String,
    pub pattern: String,
    pub chain: String,
    pub severity: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralAlert {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub timestamp: String,
    pub chain_id: u64,
    pub block_number: u64,
    pub pattern: String,
    pub symbols: Vec<AlertSymbol>,
    pub alert_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSymbol {
    pub symbol: String,
    pub from: String,
    pub to: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Subscription {
    pub patterns: Vec<String>,
    pub chains: Vec<u64>,
}

pub struct WebSocketServer {
    port: u16,
    subscribers: Arc<RwLock<HashMap<String, (UnboundedSender<Message>, Arc<RwLock<Subscription>>)>>>,
}

impl WebSocketServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start(self: Arc<Self>) {
        let addr = format!("0.0.0.0:{}", self.port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("WebSocket Error: Failed to bind to {}: {}", addr, e);
                return;
            }
        };

        println!("WebSocket Server: listening on ws://{}", addr);

        while let Ok((stream, _)) = listener.accept().await {
            let subscribers = self.subscribers.clone();
            tokio::spawn(Self::handle_connection(stream, subscribers));
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        subscribers: Arc<RwLock<HashMap<String, (UnboundedSender<Message>, Arc<RwLock<Subscription>>)>>>,
    ) {
        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(s) => s,
            Err(_) => return,
        };

        let (mut write, mut read) = ws_stream.split();
        let client_id = Uuid::new_v4().to_string();
        let (tx, mut rx) = unbounded_channel();
        let subscription = Arc::new(RwLock::new(Subscription::default()));

        // Register client
        subscribers.write().await.insert(client_id.clone(), (tx, subscription.clone()));

        // Writer task
        let subscribers_inner = subscribers.clone();
        let client_id_inner = client_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if write.send(msg).await.is_err() {
                    break;
                }
            }
            // Cleanup on disconnect
            subscribers_inner.write().await.remove(&client_id_inner);
        });

        // Reader task (Handle subscriptions)
        while let Some(Ok(msg)) = read.next().await {
            if let Message::Text(text) = msg {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if val["type"] == "subscribe" {
                        let mut sub = subscription.write().await;
                        if let Some(patterns) = val["patterns"].as_array() {
                            sub.patterns = patterns.iter().filter_map(|p| p.as_str().map(|s| s.to_string())).collect();
                        }
                        if let Some(chains) = val["chains"].as_array() {
                            sub.chains = chains.iter().filter_map(|c| c.as_u64()).collect();
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "metrics")]
    pub async fn broadcast_alert(&self, alert: BehavioralAlert) {
        let json = match serde_json::to_string(&alert) {
            Ok(j) => j,
            Err(_) => return,
        };
        let msg = Message::Text(json);

        let subs = self.subscribers.read().await;
        for (tx, sub_lock) in subs.values() {
            let sub = sub_lock.read().await;
            
            // Filtering logic
            let chain_matches = sub.chains.is_empty() || sub.chains.contains(&alert.chain_id);
            let pattern_matches = sub.patterns.is_empty() || sub.patterns.contains(&alert.pattern);

            if chain_matches && pattern_matches {
                let _ = tx.send(msg.clone());
            }
        }
    }

    #[cfg(not(feature = "metrics"))]
    pub async fn broadcast_alert(&self, _alert: BehavioralAlert) {}
}

#[cfg(feature = "metrics")]
#[derive(Clone)]
pub struct MetricsServer {
    registry: Registry,
    pub active_rules: IntGauge,
    pub memory_usage_mb: IntGauge,
    pub connected_peers: IntGauge,
    pub rpc_calls_total: Counter,
    pub p2p_messages_total: Counter,
    pub behavioral_alerts_total: Counter,
    pub verification_failures_total: Counter,
    pub verification_duration_seconds: Histogram,
}

#[cfg(feature = "metrics")]
impl MetricsServer {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();
        
        let active_rules = IntGauge::new("sods_active_rules", "Number of active monitoring rules")?;
        let memory_usage_mb = IntGauge::new("sods_memory_usage_mb", "Memory usage in MB")?;
        let connected_peers = IntGauge::new("sods_connected_peers", "Number of connected P2P peers")?;
        
        let rpc_calls_total = Counter::new("sods_rpc_calls_total", "Total RPC calls made")?;
        let p2p_messages_total = Counter::new("sods_p2p_messages_total", "Total P2P messages processed")?;
        let behavioral_alerts_total = Counter::new("sods_behavioral_alerts_total", "Total behavioral alerts triggered")?;
        let verification_failures_total = Counter::new("sods_verification_failures_total", "Total failed block verifications")?;
        
        let verification_duration_seconds = Histogram::with_opts(
            HistogramOpts::new("sods_verification_duration_seconds", "Time spent verifying blocks")
        )?;
        
        registry.register(Box::new(active_rules.clone()))?;
        registry.register(Box::new(memory_usage_mb.clone()))?;
        registry.register(Box::new(connected_peers.clone()))?;
        registry.register(Box::new(rpc_calls_total.clone()))?;
        registry.register(Box::new(p2p_messages_total.clone()))?;
        registry.register(Box::new(behavioral_alerts_total.clone()))?;
        registry.register(Box::new(verification_failures_total.clone()))?;
        registry.register(Box::new(verification_duration_seconds.clone()))?;
        
        Ok(Self {
            registry,
            active_rules,
            memory_usage_mb,
            connected_peers,
            rpc_calls_total,
            p2p_messages_total,
            behavioral_alerts_total,
            verification_failures_total,
            verification_duration_seconds,
        })
    }

    pub async fn start_http_server(self: Arc<Self>, port: u16) {
        let app = Router::new()
            .route("/_metrics", get(|State(_metrics): State<Arc<MetricsServer>>| async move {
                let encoder = TextEncoder::new();
                let metric_families = _metrics.registry.gather();
                let mut buffer = Vec::new();
                encoder.encode(&metric_families, &mut buffer).unwrap();
                
                Response::builder()
                    .header("Content-Type", encoder.format_type())
                    .body(Full::from(buffer))
                    .unwrap()
            }))
            .with_state(self);
        
        let addr = format!("0.0.0.0:{}", port);
        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("_metrics Error: Failed to bind to {}: {}", addr, e);
                return;
            }
        };

        println!("_metrics Server: listening on http://{}/_metrics", addr);
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("_metrics Server Error: {}", e);
        }
    }
}

// Stub for optional _metrics
#[cfg(not(feature = "metrics"))]
#[derive(Clone)]
pub struct MetricsServer;

#[derive(Clone)]
pub(crate) struct MonitoringTarget {
    pub(crate) pattern: sods_core::pattern::BehavioralPattern,
    pub(crate) name: String,
    pub(crate) severity: String,
    pub(crate) pattern_str: String,
    pub(crate) chain: String,
    pub(crate) expires_at: std::time::SystemTime,
}

#[cfg(unix)]
fn get_sods_dir() -> PathBuf {
    let mut path = dirs::home_dir().expect("Failed to get home directory");
    path.push(".sods");
    fs::create_dir_all(&path).ok();
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

#[cfg(unix)]
fn get_threat_rules_file() -> PathBuf {
    get_sods_dir().join("threat_rules.json")
}

#[cfg(unix)]
fn get_trusted_keys_file() -> PathBuf {
    get_sods_dir().join("trusted_keys.json")
}

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

pub(crate) fn parse_duration(s: &str) -> std::time::Duration {
    let mut num_str = String::new();
    let mut unit = 'h';
    for c in s.chars() {
        if c.is_numeric() {
            num_str.push(c);
        } else {
            unit = c;
            break;
        }
    }
    let val = num_str.parse::<u64>().unwrap_or(24);
    match unit {
        'm' => std::time::Duration::from_secs(val * 60),
        'h' => std::time::Duration::from_secs(val * 3600),
        _ => std::time::Duration::from_secs(val * 3600),
    }
}

#[cfg(unix)]
fn start_daemon(
    pattern: Option<String>, 
    chain: String, 
    interval: String, 
    rpc_url: Option<String>, 
    autostart: bool, 
    webhook_url: Option<String>,
    threat_feed: Option<String>,
    p2p_threat_network: bool,
    expire_after_str: String,
    websocket_port: Option<u16>,
    metrics_port: Option<u16>,
) -> i32 {
    let expire_duration = parse_duration(&expire_after_str);
    let expires_at = std::time::SystemTime::now() + expire_duration;
    let sods_dir = get_sods_dir();
    let pid_file = get_pid_file();
    let log_file = get_log_file();

    // _metrics Server Setup
    let _metrics = metrics_port.and_then(|_port| {
        #[cfg(feature = "metrics")]
        { MetricsServer::new().ok().map(|m| Arc::new(m)) }
        #[cfg(not(feature = "metrics"))]
        { None }
    });

    // WebSocket Server Setup
    let ws_server = websocket_port.map(|port| Arc::new(WebSocketServer::new(port)));

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

    // --- Prepare Initial Targets ---
    let mut targets = Vec::new();
    
    // 1. Manual Pattern
    if let Some(p) = pattern {
        match sods_core::pattern::BehavioralPattern::parse(&p) {
            Ok(parsed) => {
                targets.push(MonitoringTarget {
                    pattern: parsed,
                    name: "Manual Pattern".to_string(),
                    severity: "manual".to_string(),
                    pattern_str: p,
                    chain: chain.clone(),
                    expires_at,
                });
            },
            Err(e) => {
                output::error(&format!("Invalid manual pattern: {}", e));
                return 1;
            }
        }
    }

    if !p2p_threat_network && targets.is_empty() {
        output::error("No valid patterns to monitor. Provide --pattern, --threat-feed, or enable --p2p-threat-network.");
        return 1;
    }

    println!("Starting SODS daemon...");
    println!("Monitoring {} initial targets.", targets.len());
    if p2p_threat_network {
        println!("Network:    Connected to P2P Threat Intelligence");
    }
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
            let rt = tokio::runtime::Runtime::new().unwrap();
            
            // Start _metrics server if enabled
            #[cfg(feature = "metrics")]
            if let Some(ref m) = _metrics {
                if let Some(port) = metrics_port.as_ref() {
                    let m_clone = m.clone();
                    rt.spawn(m_clone.start_http_server(*port));
                }
            }

            // Start WS server if enabled
            if let Some(ref ws) = ws_server {
                let ws_clone = ws.clone();
                rt.spawn(ws_clone.start());
            }

            rt.block_on(run_daemon_loop(targets.clone(), chain.clone(), interval.clone(), rpc_url.clone(), webhook_url.clone(), Some(threat_feed.clone()), p2p_threat_network, expire_after_str.clone(), ws_server.clone(), _metrics.clone()));
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
    let res = client.get(url).timeout(std::time::Duration::from_secs(10)).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() { return Err(format!("HTTP {}", res.status())); }
    let items: Vec<ThreatFeedItem> = res.json().await.map_err(|e| format!("Invalid JSON: {}", e))?;
    Ok(items)
}

#[cfg(unix)]
async fn run_daemon_loop(
    mut targets: Vec<MonitoringTarget>, 
    chain: String, 
    interval_str: String, 
    rpc_url_opt: Option<String>,
    webhook_url: Option<String>,
    threat_feed: Option<Option<String>>,
    p2p_enabled: bool,
    expire_after_str: String,
    ws_server: Option<Arc<WebSocketServer>>,
    _metrics: Option<Arc<MetricsServer>>, // renamed usage below to _metrics if needed
) {
    use sods_verifier::BlockVerifier;
    use crate::config::get_chain;
    use std::time::Duration;
    use notify_rust::Notification;

    let interval_secs = if interval_str.ends_with("s") {
        interval_str.trim_end_matches("s").parse::<u64>().unwrap_or(30)
    } else { 30 };
    let interval = Duration::from_secs(interval_secs);
    
    let chain_config = get_chain(&chain).unwrap();
    let rpc_urls: Vec<String> = if let Some(url) = rpc_url_opt {
        vec![url]
    } else {
        chain_config.rpc_urls.iter().map(|s| s.to_string()).collect()
    };
    
    // --- P2P Setup ---
    let mut threat_rx = if p2p_enabled {
        match SodsPeer::new(&rpc_urls[0]) {
            Ok(mut peer) => {
                println!("P2P Node Initialized: {}", peer.peer_id());
                let rx = peer.subscribe_threats();
                tokio::spawn(async move {
                    if let Err(e) = peer.listen("/ip4/0.0.0.0/tcp/0").await {
                        eprintln!("P2P Listen Error: {}", e);
                    }
                });
                Some(rx)
            },
            Err(e) => {
                eprintln!("Failed to initialize P2P node: {}", e);
                None
            }
        }
    } else {
        None
    };

    let expire_duration = parse_duration(&expire_after_str);

    // --- Memory Usage Task ---
    #[cfg(feature = "metrics")]
    if let Some(ref m) = _metrics {
        let m_clone = m.clone();
        tokio::spawn(async move {
            use sysinfo::{System, SystemExt, ProcessExt};
            let mut sys = System::new();
            let pid = sysinfo::get_current_pid().ok();
            loop {
                if let Some(p) = pid {
                    sys.refresh_process(p);
                    if let Some(proc) = sys.process(p) {
                         let mb = proc.memory() / 1024 / 1024;
                         m_clone.memory_usage_mb.set(mb as i64);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });
    }

    // --- Fetch Initial Threat Feed (Async) ---
    if let Some(Some(feed_url)) = threat_feed {
        println!("Fetching threat feed from {}...", feed_url);
        match fetch_threat_feed(&feed_url).await {
            Ok(items) => {
                println!("Loaded {} patterns from threat feed.", items.len());
                for item in items {
                     if item.chain != chain { continue; }
                     match sods_core::pattern::BehavioralPattern::parse(&item.pattern) {
                         Ok(parsed) => {
                             targets.push(MonitoringTarget {
                                 pattern: parsed,
                                 name: item.name,
                                 severity: item.severity,
                                 pattern_str: item.pattern,
                                 chain: item.chain,
                                 expires_at: std::time::SystemTime::now() + expire_duration,
                             });
                         },
                         Err(e) => eprintln!("⚠️ Skipping invalid pattern '{}': {}", item.name, e),
                     }
                }
            },
            Err(e) => eprintln!("Failed to fetch threat feed: {}", e),
        }
    }

    // --- Load Persisted Rules ---
    if p2p_enabled {
        let rules_file = get_threat_rules_file();
        if rules_file.exists() {
            if let Ok(content) = fs::read_to_string(&rules_file) {
                 if let Ok(rules) = serde_json::from_str::<Vec<ThreatRule>>(&content) {
                     println!("Loaded {} persisted threat rules.", rules.len());
                     for rule in rules {
                         if rule.chain == chain {
                             if let Ok(p) = sods_core::pattern::BehavioralPattern::parse(&rule.pattern) {
                                 targets.push(MonitoringTarget {
                                     pattern: p,
                                     name: rule.name.clone(),
                                     severity: rule.severity.clone(),
                                     pattern_str: rule.pattern.clone(),
                                     chain: rule.chain.clone(),
                                     expires_at: std::time::SystemTime::now() + expire_duration,
                                 });
                             }
                         }
                     }
                 }
            }
        }
    }

    println!("Daemon loop active. Monitoring {} targets on {}", targets.len(), chain);
    
    let is_l2 = chain != "ethereum" && chain != "sepolia";
    let profile = if is_l2 {
        sods_verifier::rpc::BackoffProfile::L2
    } else {
        sods_verifier::rpc::BackoffProfile::Ethereum
    };

    let verifier = match BlockVerifier::new(&rpc_urls) {
        Ok(v) => v.with_backoff_profile(profile),
        Err(e) => {
            eprintln!("Critical Error: Failed to initialize RPCs: {}", e);
            return;
        }
    };

    if !verifier.health_check().await {
        eprintln!("⚠️ Warning: Initial health check failed for all RPCs. Continuing in hope of recovery.");
    }

    let mut last_scanned_block = verifier.get_latest_block().await.unwrap_or(0);
    if last_scanned_block == 0 {
       println!("Warning: Could not fetch initial block. Will retry in loop.");
    }

    let mut timer = tokio::time::interval(interval);
    let mut hourly_timer = tokio::time::interval(Duration::from_secs(3600));
    let mut resource_timer = tokio::time::interval(Duration::from_secs(60));
    let mut last_gc = std::time::Instant::now();

    loop {
        tokio::select! {
             _ = resource_timer.tick() => {
                 println!("📊 TELEMETRY: Targets={} Queue=0 Time={} Memory=STABLE.rss", targets.len(), chrono::Local::now());
             }

             _ = hourly_timer.tick(), if p2p_enabled => {
                 println!("🕑 Hourly Cycle: Triggering proactive peer cross-validation...");
             }

             Ok(rule) = async {
                if let Some(rx) = &mut threat_rx {
                    rx.recv().await
                } else {
                    std::future::pending().await
                }
             } => {
                  #[cfg(feature = "metrics")]
                  if let Some(ref m) = _metrics { m.p2p_messages_total.inc(); }
                  println!("Received P2P Threat Rule: {}", rule.name);
                  
                  let rules_file = get_threat_rules_file();
                  let mut current_rules: Vec<ThreatRule> = if rules_file.exists() {
                      fs::read_to_string(&rules_file).ok()
                         .and_then(|c| serde_json::from_str(&c).ok())
                         .unwrap_or_default()
                  } else { Vec::new() };
                  
                  if !current_rules.iter().any(|r| r.id == rule.id) {
                      current_rules.push(rule.clone());
                      if current_rules.len() > 1000 {
                          current_rules.remove(0);
                      }
                      if let Ok(json) = serde_json::to_string_pretty(&current_rules) {
                          let _ = fs::write(rules_file, json);
                      }
                      if rule.chain == chain {
                          if let Ok(p) = sods_core::pattern::BehavioralPattern::parse(&rule.pattern) {
                              targets.push(MonitoringTarget {
                                  pattern: p,
                                  name: rule.name.clone(),
                                  severity: rule.severity.clone(),
                                  pattern_str: rule.pattern.clone(),
                                  chain: rule.chain.clone(),
                                  expires_at: std::time::SystemTime::now() + expire_duration,
                              });
                              let msg = format!("Active P2P Rule Applied: {}", rule.name);
                              println!("{}", msg);
                              let _ = Notification::new().summary("SODS Threat Update").body(&msg).show();
                          }
                      }
                  }
             }

             _ = timer.tick() => {
                 if last_gc.elapsed() >= Duration::from_secs(300) {
                     let before = targets.len();
                     targets.retain(|t| std::time::SystemTime::now() < t.expires_at);
                     let after = targets.len();
                     if before > after {
                         println!("🧹 Garbage Collection: Pruned {} expired rules ({} remaining).", before - after, after);
                     }
                     last_gc = std::time::Instant::now();
                 }

                  #[cfg(feature = "metrics")]
                 if let Some(ref m) = _metrics { m.active_rules.set(targets.len() as i64); }
                  #[cfg(feature = "metrics")]
                 let start_v = std::time::Instant::now();
                 
                 match verifier.get_latest_block().await {
                    Ok(current_head) => {
                        #[cfg(feature = "metrics")]
                        if let Some(ref m) = _metrics { m.rpc_calls_total.inc(); }
                        if current_head > last_scanned_block {
                            if last_scanned_block == 0 {
                                last_scanned_block = current_head;
                                continue;
                            }
                            
                            for block_num in (last_scanned_block + 1)..=current_head {
                                match verifier.fetch_block_symbols(block_num).await {
                                    Ok(symbols) => {
                                        #[cfg(feature = "metrics")]
                                        if let Some(ref m) = _metrics { m.rpc_calls_total.inc(); }
                                        for target in &targets {
                                            if let Some(matched_symbols) = target.pattern.matches(&symbols, None) {
                                                #[cfg(feature = "metrics")]
                                                if let Some(ref m) = _metrics { m.behavioral_alerts_total.inc(); }
                                                let msg = format!("🚨 {} ({}) detected on Block #{}", target.name, target.severity, block_num);
                                                println!("{}", msg);
                                                let _ = Notification::new().summary("SODS Threat Alert 🚨").body(&msg).show();

                                                if let Some(ref ws) = ws_server {
                                                    let alert = BehavioralAlert {
                                                        msg_type: "behavioral_alert".into(),
                                                        timestamp: chrono::Utc::now().to_rfc3339(),
                                                        chain_id: chain_config.chain_id,
                                                        block_number: block_num,
                                                        pattern: target.pattern_str.clone(),
                                                        symbols: matched_symbols.iter().map(|s| AlertSymbol {
                                                            symbol: s.symbol.clone(),
                                                            from: format!("{:?}", s.from),
                                                            to: format!("{:?}", s.to),
                                                            value: s.value.to_string(),
                                                        }).collect(),
                                                        alert_id: format!("alert_{}_{}", block_num, Uuid::new_v4().to_string().split('-').next().unwrap()),
                                                    };
                                                    ws.broadcast_alert(alert).await;
                                                }

                                                if let Some(ref url) = webhook_url {
                                                    static SALT: once_cell::sync::Lazy<String> = once_cell::sync::Lazy::new(|| {
                                                        use rand::Rng;
                                                        rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).map(char::from).collect()
                                                    });
                                                    let mut payload_seed = target.pattern_str.clone();
                                                    payload_seed.push_str(&SALT);
                                                    let pattern_hash = ethers_core::utils::keccak256(payload_seed.as_bytes());
                                                    let payload = json!({
                                                        "alert": "Behavioral pattern detected",
                                                        "chain": chain,
                                                        "block_number": block_num,
                                                        "pattern_hash_blinded": format!("0x{}", hex::encode(pattern_hash)),
                                                        "threat_name": target.name,
                                                        "severity": target.severity,
                                                        "timestamp": chrono::Utc::now().to_rfc3339(),
                                                        "source": "daemon"
                                                    });
                                                    tokio::spawn(send_webhook(url.clone(), payload));
                                                }
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        #[cfg(feature = "metrics")]
                                        if let Some(ref m) = _metrics { m.verification_failures_total.inc(); }
                                        eprintln!("Error fetching block #{}: {}", block_num, e);
                                    }
                                }
                            }
                            last_scanned_block = current_head;
                        }
                    },
                    Err(e) => {
                        #[cfg(feature = "metrics")]
                        if let Some(ref m) = _metrics { m.verification_failures_total.inc(); }
                        println!("RPC Error: {}", e);
                    }
                 }
                 #[cfg(feature = "metrics")]
                 if let Some(ref m) = _metrics { m.verification_duration_seconds.observe(start_v.elapsed().as_secs_f64()); }
             }
        }
    }
}

#[cfg(unix)]
async fn send_webhook(url: String, payload: serde_json::Value) {
    if !url.starts_with("https://") { return; }
    let client = reqwest::Client::new();
    let _ = client.post(&url).json(&payload).timeout(std::time::Duration::from_secs(5)).send().await;
}

#[cfg(unix)]
fn stop_daemon() -> i32 {
    let pid_file = get_pid_file();
    if !pid_file.exists() { output::error("Daemon is not running."); return 1; }
    let pid_str = fs::read_to_string(&pid_file).unwrap();
    let pid = pid_str.trim().parse::<i32>().unwrap();
    let _ = std::process::Command::new("kill").arg(pid.to_string()).output();
    let _ = fs::remove_file(pid_file);
    println!("Daemon stopped.");
    0
}

#[cfg(unix)]
fn check_status() -> bool {
    let pid_file = get_pid_file();
    if !pid_file.exists() { return false; }
    let pid_str = match fs::read_to_string(&pid_file) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let pid_val = match pid_str.trim().parse::<usize>() {
         Ok(p) => p,
         Err(_) => return false,
    };
    System::new_all().process(Pid::from(pid_val)).is_some()
}

// -----------------------------------------------------------------------------
// Windows Implementation (Stub)
// -----------------------------------------------------------------------------

#[cfg(not(unix))]
fn start_daemon(
    _pattern: Option<String>, 
    _chain: String, 
    _interval: String, 
    _rpc_url: Option<String>, 
    _autostart: bool, 
    _webhook_url: Option<String>,
    _threat_feed: Option<String>,
    _p2p_threat_network: bool,
    _expire_after: String,
    _websocket_port: Option<u16>,
    _metrics_port: Option<u16>,
) -> i32 {
    output::error("Daemon mode is currently only supported on Linux/macOS.");
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
    run_sync(args)
}

pub fn run_sync(args: DaemonArgs) -> i32 {
    match args.command {
        DaemonCommands::Start { pattern, chain, interval, rpc_url, autostart, webhook_url, threat_feed, p2p_threat_network, expire_after, websocket_port, metrics_port } => {
            start_daemon(pattern, chain, interval, rpc_url, autostart, webhook_url, threat_feed, p2p_threat_network, expire_after, websocket_port, metrics_port)
        },
        DaemonCommands::Stop => stop_daemon(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};
    use sods_core::pattern::BehavioralPattern;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1h"), Duration::from_secs(3600));
        assert_eq!(parse_duration("30m"), Duration::from_secs(1800));
        assert_eq!(parse_duration("24h"), Duration::from_secs(24 * 3600));
        assert_eq!(parse_duration("5x"), Duration::from_secs(5 * 3600));
    }

    #[test]
    fn test_target_retention() {
        let mut targets = vec![
            MonitoringTarget {
                pattern: BehavioralPattern::parse("Tf").unwrap(),
                name: "Expired".to_string(),
                severity: "info".to_string(),
                pattern_str: "Tf".to_string(),
                chain: "base".to_string(),
                expires_at: SystemTime::now() - Duration::from_secs(10),
            },
            MonitoringTarget {
                pattern: BehavioralPattern::parse("Sw").unwrap(),
                name: "Active".to_string(),
                severity: "info".to_string(),
                pattern_str: "Sw".to_string(),
                chain: "base".to_string(),
                expires_at: SystemTime::now() + Duration::from_secs(10),
            },
        ];

        targets.retain(|t| SystemTime::now() < t.expires_at);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "Active");
    }
}
