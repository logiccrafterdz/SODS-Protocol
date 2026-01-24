use clap::Args;
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use colored::Colorize;
use crate::output;
use serde_json::json;

#[derive(Args)]
pub struct ListenArgs {
    /// WebSocket URL to connect to (e.g., ws://localhost:8080)
    #[arg(short, long, default_value = "ws://localhost:8080")]
    pub websocket: String,

    /// Filter alerts by pattern (local and remote)
    #[arg(short, long)]
    pub pattern: Option<String>,
}

pub async fn run(args: ListenArgs) -> i32 {
    output::info(&format!("Connecting to WebSocket server: {}...", args.websocket));

    let (ws_stream, _) = match connect_async(&args.websocket).await {
        Ok(s) => s,
        Err(e) => {
            output::error(&format!("Failed to connect: {}", e));
            return 1;
        }
    };

    output::success("Connected! Waiting for live behavioral alerts...");

    let (mut write, mut read) = ws_stream.split();

    // Send subscription if pattern is provided
    if let Some(ref pattern) = args.pattern {
        let sub_msg = json!({
            "type": "subscribe",
            "patterns": [pattern]
        });
        if let Ok(text) = serde_json::to_string(&sub_msg) {
            let _ = write.send(Message::Text(text)).await;
            output::info(&format!("Subscribed to pattern: {}", pattern.cyan()));
        }
    }

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(alert) = serde_json::from_str::<serde_json::Value>(&text) {
                    println!("\n{}", "🔔 NEW BEHAVIORAL ALERT".yellow().bold());
                    println!("   Chain ID:   {}", alert["chain_id"]);
                    println!("   Block:      {}", alert["block_number"]);
                    println!("   Pattern:    {}", alert["pattern"].as_str().unwrap_or("unknown").cyan());
                    println!("   Timestamp:  {}", alert["timestamp"]);
                    println!("   Alert ID:   {}", alert["alert_id"]);
                    
                    if let Some(symbols) = alert["symbols"].as_array() {
                        println!("   Symbols:");
                        for sym in symbols {
                            println!("     - {} from {} to {} (value: {})", 
                                sym["symbol"].as_str().unwrap_or("?").green(),
                                sym["from"].as_str().unwrap_or("?").dimmed(),
                                sym["to"].as_str().unwrap_or("?").dimmed(),
                                sym["value"].as_str().unwrap_or("0")
                            );
                        }
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                output::error(&format!("Connection error: {}", e));
                break;
            }
        }
    }

    0
}
