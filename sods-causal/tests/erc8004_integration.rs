use ethers::types::Address;
use sods_causal::{
    CausalEvent, CausalMerkleTree, AgentBehaviorPattern, 
    generate_behavioral_proof,
    ReputationFeedback, ValidationRequest, ValidationHandler
};

#[test]
fn test_agent_registration_json_valid() {
    // This is tested via the CLI command in E2E, but we can check the struct here if exposed.
    // For now, let's focus on the causal/ERC8004 logic.
}

#[test]
fn test_feedback_parsing_and_storage() {
    let feedback = ReputationFeedback {
        tag1: "behavioral_proof_accuracy".to_string(),
        tag2: "integration_test".to_string(),
        value: 95,
        metadata: Some("High accuracy matched".to_string()),
    };

    assert!(feedback.validate().is_ok());
    
    let response = feedback.generate_response();
    assert!(response.contains("95"));
}

#[test]
fn test_validation_request_handling() {
    let agent_id = Address::random();
    let events = vec![
        CausalEvent::builder()
            .agent_id(agent_id)
            .nonce(0)
            .sequence_index(0)
            .event_type("task_executed")
            .result("success")
            .timestamp(1000)
            .build()
            .unwrap()
    ];
    
    let tree = CausalMerkleTree::new(events).unwrap();
    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 1,
        max_count: None,
        time_window: None,
    };

    let proof = generate_behavioral_proof(&tree, &pattern, 1500).unwrap();
    let proof_bytes = serde_json::to_vec(&proof).unwrap();

    let request = ValidationRequest {
        request_id: [1u8; 32].into(),
        agent_id,
        proof_data: proof_bytes,
        timestamp: 1500,
    };

    let response = ValidationHandler::handle_request(request).unwrap();
    assert_eq!(response.score, 100);
    assert!(response.metadata.contains("successfully verified"));
}

#[tokio::test]
async fn test_full_agent_lifecycle_mock() {
    // 1. Setup History
    let agent_id = Address::repeat_byte(0x42);
    let events = vec![
        CausalEvent::builder()
            .agent_id(agent_id)
            .nonce(0)
            .sequence_index(0)
            .event_type("task_executed")
            .result("success")
            .timestamp(100)
            .build()
            .unwrap()
    ];
    let tree = CausalMerkleTree::new(events).unwrap();

    // 2. Generate Proof for matching
    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 1,
        max_count: None,
        time_window: None,
    };
    let proof = generate_behavioral_proof(&tree, &pattern, 200).unwrap();

    // 3. Verify Proof
    assert!(proof.verify(200));

    // 4. Submit Feedback
    let feedback = ReputationFeedback {
        tag1: "behavioral_proof_accuracy".to_string(),
        tag2: "".to_string(),
        value: 100,
        metadata: None,
    };
    assert!(feedback.validate().is_ok());
}
