use ethers::types::Address;
use sods_causal::{CausalEvent, CausalMerkleTree, AgentBehaviorPattern, generate_behavioral_proof};

fn create_event(nonce: u64, seq: u32, result: &str, timestamp: u64) -> CausalEvent {
    CausalEvent::builder()
        .agent_id(Address::repeat_byte(1))
        .nonce(nonce)
        .sequence_index(seq)
        .event_type("task_executed")
        .result(result)
        .timestamp(timestamp)
        .build()
        .unwrap()
}

#[test]
fn test_generate_and_verify_success_pattern() {
    let events = vec![
        create_event(0, 0, "success", 100),
        create_event(0, 1, "success", 200),
        create_event(1, 0, "failure", 300),
        create_event(1, 1, "success", 400),
    ];

    let tree = CausalMerkleTree::new(events).unwrap();
    
    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 3,
        max_count: None,
        time_window: None,
    };

    let proof = generate_behavioral_proof(&tree, &pattern, 500).unwrap();
    assert!(proof.verify(500));
}

#[test]
fn test_tampered_event_in_proof_fails() {
    let events = vec![
        create_event(0, 0, "success", 100),
        create_event(0, 1, "success", 200),
    ];

    let tree = CausalMerkleTree::new(events).unwrap();
    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 2,
        max_count: None,
        time_window: None,
    };

    let mut proof = generate_behavioral_proof(&tree, &pattern, 500).unwrap();
    
    // Tamper with one of the matched events
    proof.matched_events[0].result = "failure".to_string();

    assert!(!proof.verify(500));
}

#[test]
fn test_insufficient_events_fails() {
    let events = vec![
        create_event(0, 0, "success", 100),
        create_event(0, 1, "failure", 200),
    ];

    let tree = CausalMerkleTree::new(events).unwrap();
    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 2, // Requires 2, only 1 available
        max_count: None,
        time_window: None,
    };

    let result = generate_behavioral_proof(&tree, &pattern, 500);
    assert!(result.is_err());
}

#[test]
fn test_cross_nonce_pattern() {
    let events = vec![
        create_event(0, 0, "success", 100),
        create_event(1, 0, "success", 200),
    ];

    let tree = CausalMerkleTree::new(events).unwrap();
    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 2,
        max_count: None,
        time_window: None,
    };

    let proof = generate_behavioral_proof(&tree, &pattern, 300).unwrap();
    assert!(proof.verify(300));
}
