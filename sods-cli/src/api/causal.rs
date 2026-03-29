use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use ethers::types::Address;
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;

use sods_causal::{
    generate_behavioral_proof, AgentBehaviorPattern, CausalBehavioralProof, CausalEventRecorder,
    ReputationClaim, ReputationFeedback,
};

use crate::logging::ValidationLog;
use crate::monitoring::metrics::AgentMetrics;
use tracing::info;

use crate::api::health::MonitoringState;
use axum::extract::FromRef;

/// Shared state for the Causal API server.
pub struct ApiState {
    pub recorder: Arc<tokio::sync::RwLock<CausalEventRecorder>>,
    pub metrics: Option<Arc<AgentMetrics>>,
}

impl FromRef<Arc<ApiState>> for MonitoringState {
    fn from_ref(state: &Arc<ApiState>) -> Self {
        MonitoringState {
            metrics: state.metrics.clone(),
        }
    }
}

/// Request for proof generation.
#[derive(Debug, Deserialize)]
pub struct ProofRequest {
    pub pattern: AgentBehaviorPattern,
    pub now: u64,
}

/// Accepts a CausalBehavioralProof and returns verification result.
async fn verify_proof(Json(proof): Json<CausalBehavioralProof>) -> impl IntoResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let is_valid = proof.verify(now);

    info!(target: "validation", 
          event = "validation_completed",
          result = if is_valid { "success" } else { "failure" },
          "Validation request processed");

    Json(serde_json::json!({
        "valid": is_valid,
        "timestamp": now,
        "message": if is_valid { "Proof verified successfully" } else { "Proof verification failed" }
    }))
}

/// Returns a pre-generated behavioral proof for an agent.
async fn get_proof(
    Path(agent_id): Path<String>,
    State(state): State<Arc<ApiState>>,
    Json(req): Json<ProofRequest>,
) -> impl IntoResponse {
    let agent_addr: Address = match agent_id.parse() {
        Ok(addr) => addr,
        Err(_) => {
            return (axum::http::StatusCode::BAD_REQUEST, "Invalid agent address").into_response()
        }
    };

    let recorder = state.recorder.read().await;
    if let Some(ref m) = state.metrics {
        m.validation_requests_received_total.inc();
    }

    let start_time = std::time::Instant::now();
    let result = match recorder.build_merkle_tree(&agent_addr) {
        Ok(tree) => match generate_behavioral_proof(&tree, &req.pattern, req.now) {
            Ok(proof) => {
                if let Some(ref m) = state.metrics {
                    m.validation_responses_submitted_total.inc();
                }
                Json(proof).into_response()
            }
            Err(e) => (
                axum::http::StatusCode::NOT_FOUND,
                format!("Failed to generate proof: {}", e),
            )
                .into_response(),
        },
        Err(e) => (
            axum::http::StatusCode::NOT_FOUND,
            format!("Agent history not found: {}", e),
        )
            .into_response(),
    };

    info!(target: "validation",
          log = ?ValidationLog {
              timestamp: crate::logging::now_iso(),
              level: "info".to_string(),
              event: "proof_generated".to_string(),
              agent_id: agent_id.clone(),
              request_hash: "0x...".to_string(), // In a real app, this would be the request's hash
              result: "success".to_string(),
              duration_ms: start_time.elapsed().as_millis() as u64,
              error_message: None,
          },
          "Behavioral proof generation completed");

    result
}

/// Accept client feedback and forward to Reputation Registry logic.
async fn submit_feedback(Json(feedback): Json<ReputationFeedback>) -> impl IntoResponse {
    if let Err(e) = feedback.validate() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid feedback: {}", e),
        )
            .into_response();
    }

    // In a real implementation, this would submit to an on-chain registry or IPFS.
    // For now, we return a successful reputation claim as a mock.
    info!(target: "reputation",
          event = "feedback_submitted",
          score = feedback.value,
          "Reputation feedback received");

    let claim = ReputationClaim::new(feedback, "ipfs://mock-hash".to_string());

    Json(claim).into_response()
}

pub async fn start_server(
    port: u16,
    metrics: Option<Arc<AgentMetrics>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(ApiState {
        recorder: Arc::new(tokio::sync::RwLock::new(CausalEventRecorder::new())),
        metrics,
    });

    let app = Router::new()
        .merge(crate::api::health::router::<Arc<ApiState>>())
        .route("/causal/verify", post(verify_proof))
        .route("/causal/proof/:agent_id", post(get_proof))
        .route("/causal/feedback", post(submit_feedback))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!("🚀 Causal API Server listening on http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
