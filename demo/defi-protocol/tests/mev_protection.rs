use ethers::types::Address;
use sods_causal::{
    CausalEvent, CausalEventRecorder, AgentBehaviorPattern,
    generate_behavioral_proof, CausalBehavioralProof
};

#[tokio::test]
async fn test_valid_trading_behavior_unlocks_escrow_mock() {
    let agent_id = Address::random();
    let mut recorder = CausalEventRecorder::new();

    // 1. Record 10 profitable trades
    for i in 0..10 {
        let event = CausalEvent::builder()
            .agent_id(agent_id)
            .nonce(0)
            .sequence_index(i as u32)
            .event_type("trade_executed")
            .result("profit")
            .timestamp(1000 + (i as u64 * 10))
            .build()
            .unwrap();
        recorder.record_event(event).unwrap();
    }

    let tree = recorder.build_merkle_tree(&agent_id).unwrap();
    let pattern = AgentBehaviorPattern {
        event_type: "trade_executed".to_string(),
        result_filter: "profit".to_string(),
        min_count: 10,
        max_count: None,
        time_window: None,
    };

    // 2. Generate Proof
    let now = 2000;
    let proof = generate_behavioral_proof(&tree, &pattern, now).unwrap();

    // 3. Verify Proof (Simulating Escrow contract logic)
    assert!(proof.verify(now), "Valid history should produce valid proof");
}

#[tokio::test]
async fn test_malicious_trading_behavior_blocks_escrow_mock() {
    let agent_id = Address::random();
    let mut recorder = CausalEventRecorder::new();

    // 1. Record 9 profitable trades and 1 loss (MEV exploited)
    for i in 0..9 {
        let event = CausalEvent::builder()
            .agent_id(agent_id)
            .nonce(0)
            .sequence_index(i as u32)
            .event_type("trade_executed")
            .result("profit")
            .timestamp(1000 + (i as u64 * 10))
            .build()
            .unwrap();
        recorder.record_event(event).unwrap();
    }
    
    // The 10th one is a loss
    let event = CausalEvent::builder()
        .agent_id(agent_id)
        .nonce(0)
        .sequence_index(9)
        .event_type("trade_executed")
        .result("loss")
        .timestamp(1100)
        .build()
        .unwrap();
    recorder.record_event(event).unwrap();

    let tree = recorder.build_merkle_tree(&agent_id).unwrap();
    let pattern = AgentBehaviorPattern {
        event_type: "trade_executed".to_string(),
        result_filter: "profit".to_string(),
        min_count: 10,
        max_count: None,
        time_window: None,
    };

    // 2. Attempt to generate proof for 10 profits (should fail or return insufficient)
    let now = 2000;
    let proof_res = generate_behavioral_proof(&tree, &pattern, now);
    
    // In our implementation, generate_behavioral_proof returns an error if count is not met
    assert!(proof_res.is_err(), "Should fail to generate proof for 10 profits when only 9 exist");
}

#[test]
fn test_tampered_proof_detection() {
    let agent_id = Address::random();
    let mut recorder = CausalEventRecorder::new();

    for i in 0..10 {
        let event = CausalEvent::builder()
            .agent_id(agent_id)
            .nonce(0)
            .sequence_index(i as u32)
            .event_type("trade_executed")
            .result("profit")
            .timestamp(1000 + (i as u64 * 10))
            .build()
            .unwrap();
        recorder.record_event(event).unwrap();
    }

    let tree = recorder.build_merkle_tree(&agent_id).unwrap();
    let pattern = AgentBehaviorPattern {
        event_type: "trade_executed".to_string(),
        result_filter: "profit".to_string(),
        min_count: 10,
        max_count: None,
        time_window: None,
    };

    let now = 2000;
    let proof = generate_behavioral_proof(&tree, &pattern, now).unwrap();

    // Tamper with one event in the proof
    let mut tampered_events = proof.matched_events.clone();
    tampered_events[0].nonce = 999; 

    let tampered_proof = CausalBehavioralProof {
        pattern: proof.pattern.clone(),
        matched_events: tampered_events,
        event_proofs: proof.event_proofs.clone(),
        agent_root: proof.agent_root,
    };

    assert!(!tampered_proof.verify(now), "Tampered proof must fail verification");
}
