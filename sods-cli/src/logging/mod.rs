use chrono::Utc;
use serde::Serialize;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Serialize, Debug)]
pub struct ValidationLog {
    pub timestamp: String,
    pub level: String,
    pub event: String,
    pub agent_id: String,
    pub request_hash: String,
    pub result: String,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json().with_target(true))
        .init();
}

pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
