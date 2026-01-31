use axum::{Router, routing::post, Json, extract::{Path, State}, response::IntoResponse};
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use ethers::types::Address;

use sods_causal::{
    CausalBehavioralProof, CausalEventRecorder, AgentBehaviorPattern,
    ReputationFeedback, ReputationClaim,
    generate_behavioral_proof
};

/// Shared state for the Causal API server.
pub struct ApiState {
    pub recorder: Arc<tokio::sync::RwLock<CausalEventRecorder>>,
}

/// Request for proof generation.
#[derive(Debug, Deserialize)]
pub struct ProofRequest {
    pub pattern: AgentBehaviorPattern,
    pub now: u64,
}

/// Accepts a CausalBehavioralProof and returns verification result.
async fn verify_proof(
    Json(proof): Json<CausalBehavioralProof>,
) -> impl IntoResponse {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let is_valid = proof.verify(now);
    
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
        Err(_) => return (axum::http::StatusCode::BAD_REQUEST, "Invalid agent address").into_response(),
    };

    let recorder = state.recorder.read().await;
    match recorder.build_merkle_tree(&agent_addr) {
        Ok(tree) => {
            match generate_behavioral_proof(&tree, &req.pattern, req.now) {
                Ok(proof) => Json(proof).into_response(),
                Err(e) => (axum::http::StatusCode::NOT_FOUND, format!("Failed to generate proof: {}", e)).into_response(),
            }
        }
        Err(e) => (axum::http::StatusCode::NOT_FOUND, format!("Agent history not found: {}", e)).into_response(),
    }
}

/// Accept client feedback and forward to Reputation Registry logic.
async fn submit_feedback(
    Json(feedback): Json<ReputationFeedback>,
) -> impl IntoResponse {
    if let Err(e) = feedback.validate() {
        return (axum::http::StatusCode::BAD_REQUEST, format!("Invalid feedback: {}", e)).into_response();
    }

    // In a real implementation, this would submit to an on-chain registry or IPFS.
    // For now, we return a successful reputation claim as a mock.
    let claim = ReputationClaim::new(feedback, "ipfs://mock-hash".to_string());
    
    Json(claim).into_response()
}

pub async fn start_server(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(ApiState {
        recorder: Arc::new(tokio::sync::RwLock::new(CausalEventRecorder::new())),
    });

    let app = Router::new()
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
