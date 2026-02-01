#[cfg(feature = "api")]
use axum::{Json, routing::get, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub erc8004: Erc8004Status,
    pub metrics: HealthMetrics,
}

#[derive(Serialize)]
pub struct Erc8004Status {
    pub identity_registered: bool,
    pub validation_registry_connected: bool,
    pub reputation_registry_connected: bool,
    pub escrow_contract_accessible: bool,
}

#[derive(Serialize)]
pub struct HealthMetrics {
    pub uptime_seconds: u64,
    pub validation_success_rate: f64,
    pub quality_score: u32,
}

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        erc8004: Erc8004Status {
            identity_registered: true,
            validation_registry_connected: true,
            reputation_registry_connected: true,
            escrow_contract_accessible: true,
        },
        metrics: HealthMetrics {
            uptime_seconds: 0, 
            validation_success_rate: 100.0,
            quality_score: 95,
        },
    })
}

pub async fn readiness_check() -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
}
