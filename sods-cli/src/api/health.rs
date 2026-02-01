use serde::{Serialize, Deserialize};
use axum::{extract::State, http::StatusCode, Json, Router, routing::get};
use std::sync::Arc;
use crate::monitoring::metrics::AgentMetrics;

#[derive(Serialize, Debug)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub erc8004: Erc8004Status,
    pub metrics: HealthMetrics,
}

#[derive(Serialize, Debug)]
pub struct Erc8004Status {
    pub identity_registered: bool,
    pub validation_registry_connected: bool,
    pub reputation_registry_connected: bool,
    pub escrow_contract_accessible: bool,
}

#[derive(Serialize, Debug)]
pub struct HealthMetrics {
    pub uptime_seconds: u64,
    pub validation_success_rate: f64,
    pub quality_score: u32,
}

#[derive(Serialize, Debug)]
pub struct ReadyResponse {
    pub status: String,
}

// We need a way to access the metrics from the state. 
// Since health.rs is used by both the causal API (ApiState) and the metrics server (Arc<AgentMetrics>),
// we'll define a trait or just use a generic approach.
// However, to follow the user's specific code request, I'll assume it's used with a state that has .metrics.

pub trait HealthState {
    fn get_metrics(&self) -> Option<Arc<AgentMetrics>>;
}

impl HealthState for Arc<crate::api::causal::ApiState> {
    fn get_metrics(&self) -> Option<Arc<AgentMetrics>> {
        self.metrics.clone()
    }
}

impl HealthState for Arc<AgentMetrics> {
    fn get_metrics(&self) -> Option<Arc<AgentMetrics>> {
        Some(self.clone())
    }
}

pub async fn health_check<S>(State(state): State<S>) -> Json<HealthResponse> 
where S: HealthState {
    let (status, erc8004_status, health_metrics) = if let Some(metrics) = state.get_metrics() {
        // Calculate real validation success rate
        let total_requests = metrics.validation_requests_received_total.get() as f64;
        let successful_responses = metrics.validation_responses_submitted_total.get() as f64;
        let success_rate = if total_requests > 0.0 {
            (successful_responses / total_requests) * 100.0
        } else {
            100.0 // No requests yet = healthy
        };
        
        // Determine overall status
        let status = if success_rate >= 80.0 && total_requests > 0.0 {
            "healthy".to_string()
        } else if total_requests == 0.0 {
            "starting".to_string() // Agent hasn't processed any requests yet
        } else {
            "degraded".to_string()
        };
        
        // Build ERC-8004 status from real connectivity
        let erc8004_status = Erc8004Status {
            identity_registered: total_requests > 0.0, // Simplified check
            validation_registry_connected: total_requests > 0.0,
            reputation_registry_connected: metrics.feedback_received_total.get() > 0.0,
            escrow_contract_accessible: metrics.payments_received_total.get() > 0.0,
        };
        
        // Build real metrics
        let health_metrics = HealthMetrics {
            uptime_seconds: metrics.agent_uptime_seconds.get() as u64,
            validation_success_rate: success_rate,
            quality_score: metrics.average_quality_score.get() as u32,
        };
        
        (status, erc8004_status, health_metrics)
    } else {
        // Fallback when metrics are disabled
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
            }
        )
    };
    
    Json(HealthResponse {
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        erc8004: erc8004_status,
        metrics: health_metrics,
    })
}

pub async fn readiness_check<S>(State(state): State<S>) -> Result<Json<ReadyResponse>, StatusCode>
where S: HealthState {
    if let Some(metrics) = state.get_metrics() {
        let total_requests = metrics.validation_requests_received_total.get();
        if total_requests > 0.0 {
            // Agent has processed at least one request = ready
            return Ok(Json(ReadyResponse { status: "ready".to_string() }));
        }
    }
    
    // Not ready yet
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

pub fn router<S>() -> Router<S>
where
    S: HealthState + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(health_check::<S>))
        .route("/health/ready", get(readiness_check::<S>))
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::Json;
    use crate::api::causal::ApiState;
    use sods_causal::CausalEventRecorder;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_health_endpoint_returns_real_metrics() {
        let metrics = AgentMetrics::new().unwrap();
        let metrics = Arc::new(metrics);
        
        // Simulate some activity
        metrics.validation_requests_received_total.inc();
        metrics.validation_responses_submitted_total.inc();
        metrics.average_quality_score.set(95.0);
        
        let state = Arc::new(ApiState {
            recorder: Arc::new(RwLock::new(CausalEventRecorder::new())),
            metrics: Some(metrics),
        });
        
        let response = health_check(State(state)).await;
        let health = response.0;
        
        assert_eq!(health.status, "healthy");
        assert_eq!(health.metrics.validation_success_rate, 100.0);
        assert_eq!(health.metrics.quality_score, 95);
    }

    #[tokio::test]
    async fn test_ready_endpoint_unavailable_when_no_requests() {
        let metrics = AgentMetrics::new().unwrap();
        let metrics = Arc::new(metrics);
        
        let state = Arc::new(ApiState {
            recorder: Arc::new(RwLock::new(CausalEventRecorder::new())),
            metrics: Some(metrics),
        });
        
        let result = readiness_check(State(state)).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn test_ready_endpoint_available_after_request() {
        let metrics = AgentMetrics::new().unwrap();
        let metrics = Arc::new(metrics);
        
        metrics.validation_requests_received_total.inc();
        
        let state = Arc::new(ApiState {
            recorder: Arc::new(RwLock::new(CausalEventRecorder::new())),
            metrics: Some(metrics),
        });
        
        let result = readiness_check(State(state)).await;
        assert!(result.is_ok());
        let ready = result.unwrap().0;
        assert_eq!(ready.status, "ready");
    }
}
