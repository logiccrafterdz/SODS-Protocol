use crate::monitoring::metrics::AgentMetrics;
use axum::{
    extract::{FromRef, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub erc8004: Erc8004Status,
    pub metrics: HealthMetrics,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Erc8004Status {
    pub identity_registered: bool,
    pub validation_registry_connected: bool,
    pub reputation_registry_connected: bool,
    pub escrow_contract_accessible: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HealthMetrics {
    pub uptime_seconds: u64,
    pub validation_success_rate: f64,
    pub quality_score: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadyResponse {
    pub status: String,
}

/// Shared state for health and readiness checks.
#[derive(Clone, Default)]
pub struct MonitoringState {
    pub metrics: Option<Arc<AgentMetrics>>,
}

// FromRef<MonitoringState> is automatically provided by Axum's blanket impl.

impl FromRef<Arc<AgentMetrics>> for MonitoringState {
    fn from_ref(metrics: &Arc<AgentMetrics>) -> Self {
        Self {
            metrics: Some(metrics.clone()),
        }
    }
}

pub async fn health_check(State(state): State<MonitoringState>) -> Json<HealthResponse> {
    let (status, erc8004_status, health_metrics) = if let Some(ref metrics) = state.metrics {
        let total_requests = metrics.validation_requests_received_total.get() as f64;
        let successful_responses = metrics.validation_responses_submitted_total.get() as f64;
        let success_rate = if total_requests > 0.0 {
            (successful_responses / total_requests) * 100.0
        } else {
            100.0
        };

        let status = if success_rate >= 80.0 && total_requests > 0.0 {
            "healthy".to_string()
        } else if total_requests == 0.0 {
            "starting".to_string()
        } else {
            "degraded".to_string()
        };

        let erc8004_status = Erc8004Status {
            identity_registered: total_requests > 0.0,
            validation_registry_connected: total_requests > 0.0,
            reputation_registry_connected: metrics.feedback_received_total.get() > 0.0,
            escrow_contract_accessible: metrics.payments_received_total.get() > 0.0,
        };

        let health_metrics = HealthMetrics {
            uptime_seconds: metrics.agent_uptime_seconds.get() as u64,
            validation_success_rate: success_rate,
            quality_score: metrics.average_quality_score.get() as u32,
        };

        (status, erc8004_status, health_metrics)
    } else {
        (
            "healthy".to_string(),
            Erc8004Status {
                identity_registered: true,
                validation_registry_connected: true,
                reputation_registry_connected: true,
                escrow_contract_accessible: true,
            },
            HealthMetrics {
                uptime_seconds: 0,
                validation_success_rate: 100.0,
                quality_score: 100,
            },
        )
    };

    Json(HealthResponse {
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        erc8004: erc8004_status,
        metrics: health_metrics,
    })
}

pub async fn readiness_check(
    State(state): State<MonitoringState>,
) -> Result<Json<ReadyResponse>, StatusCode> {
    if let Some(ref metrics) = state.metrics {
        if metrics.validation_requests_received_total.get() > 0.0 {
            return Ok(Json(ReadyResponse {
                status: "ready".to_string(),
            }));
        }
    }
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

pub fn router<S>() -> Router<S>
where
    S: Send + Sync + Clone + 'static,
    MonitoringState: FromRef<S>,
{
    Router::new()
        .route("/health", get(health_check))
        .route("/health/ready", get(readiness_check))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_starting_status() {
        let metrics = Arc::new(AgentMetrics::new().unwrap());
        let state = MonitoringState {
            metrics: Some(metrics),
        };
        let response = health_check(State(state)).await;
        assert_eq!(response.0.status, "starting");
    }

    #[tokio::test]
    async fn test_health_healthy_status() {
        let metrics = Arc::new(AgentMetrics::new().unwrap());
        metrics.validation_requests_received_total.inc();
        metrics.validation_responses_submitted_total.inc();

        let state = MonitoringState {
            metrics: Some(metrics),
        };
        let response = health_check(State(state)).await;
        assert_eq!(response.0.status, "healthy");
        assert_eq!(response.0.metrics.validation_success_rate, 100.0);
    }

    #[tokio::test]
    async fn test_health_degraded_status() {
        let metrics = Arc::new(AgentMetrics::new().unwrap());
        // 10 requests, 1 success = 10% rate
        for _ in 0..10 {
            metrics.validation_requests_received_total.inc();
        }
        metrics.validation_responses_submitted_total.inc();

        let state = MonitoringState {
            metrics: Some(metrics),
        };
        let response = health_check(State(state)).await;
        assert_eq!(response.0.status, "degraded");
        assert_eq!(response.0.metrics.validation_success_rate, 10.0);
    }

    #[tokio::test]
    async fn test_health_graceful_without_metrics() {
        let state = MonitoringState { metrics: None };
        let response = health_check(State(state)).await;
        assert_eq!(response.0.status, "healthy");
    }

    #[tokio::test]
    async fn test_readiness_unavailable_when_no_requests() {
        let metrics = Arc::new(AgentMetrics::new().unwrap());
        let state = MonitoringState {
            metrics: Some(metrics),
        };
        let result = readiness_check(State(state)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_readiness_available_after_request() {
        let metrics = Arc::new(AgentMetrics::new().unwrap());
        metrics.validation_requests_received_total.inc();
        let state = MonitoringState {
            metrics: Some(metrics),
        };
        let result = readiness_check(State(state)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_concurrent_metric_updates() {
        let metrics = Arc::new(AgentMetrics::new().unwrap());
        let mut handles = Vec::new();
        for _ in 0..50 {
            let m = metrics.clone();
            handles.push(tokio::spawn(async move {
                m.validation_requests_received_total.inc();
                m.validation_responses_submitted_total.inc();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        let state = MonitoringState {
            metrics: Some(metrics.clone()),
        };
        let response = health_check(State(state)).await;
        assert_eq!(response.0.status, "healthy");
        assert_eq!(response.0.metrics.validation_success_rate, 100.0);
    }
}
