//! Unit tests for the causal event model.
//!
//! These tests verify the core functionality of `CausalEvent` including
//! serialization, ordering, and validation.

use ethers::types::{Address, H256};
use sods_causal::{CausalError, CausalEvent, VALID_RESULTS};

fn test_address() -> Address {
    "0x1234567890123456789012345678901234567890"
        .parse()
        .unwrap()
}

/// Test that events can be serialized to JSON and back without loss.
#[test]
fn test_causal_event_serialization_roundtrip() {
    let metadata_hash: H256 = "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        .parse()
        .unwrap();

    let event = CausalEvent::builder()
        .agent_id(test_address())
        .nonce(42)
        .sequence_index(3)
        .event_type("task_executed")
        .task_id("task-abc-123")
        .result("success")
        .timestamp(1700000000)
        .metadata_hash(metadata_hash)
        .build()
        .unwrap();

    // Serialize to JSON
    let json = serde_json::to_string(&event).unwrap();

    // Deserialize back
    let deserialized: CausalEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(event, deserialized);
    assert_eq!(deserialized.agent_id, test_address());
    assert_eq!(deserialized.nonce, 42);
    assert_eq!(deserialized.sequence_index, 3);
    assert_eq!(deserialized.task_id, Some("task-abc-123".to_string()));
    assert_eq!(deserialized.metadata_hash, Some(metadata_hash));
}

/// Test that causal ordering works correctly across various scenarios.
#[test]
fn test_causal_event_ordering_correctness() {
    let events: Vec<CausalEvent> = vec![
        // Nonce 0, seq 0
        CausalEvent::builder()
            .agent_id(test_address())
            .nonce(0)
            .sequence_index(0)
            .build()
            .unwrap(),
        // Nonce 0, seq 1
        CausalEvent::builder()
            .agent_id(test_address())
            .nonce(0)
            .sequence_index(1)
            .build()
            .unwrap(),
        // Nonce 1, seq 0
        CausalEvent::builder()
            .agent_id(test_address())
            .nonce(1)
            .sequence_index(0)
            .build()
            .unwrap(),
        // Nonce 2, seq 0
        CausalEvent::builder()
            .agent_id(test_address())
            .nonce(2)
            .sequence_index(0)
            .build()
            .unwrap(),
    ];

    // Verify ordering is correct
    for i in 0..events.len() - 1 {
        assert!(
            events[i] < events[i + 1],
            "Event {} should be less than event {}",
            i,
            i + 1
        );
    }

    // Test sorting a shuffled vector
    let mut shuffled = vec![
        events[3].clone(),
        events[0].clone(),
        events[2].clone(),
        events[1].clone(),
    ];
    shuffled.sort();

    assert_eq!(shuffled, events);
}

/// Test that sequence gaps are properly detected.
#[test]
fn test_sequence_gap_detection() {
    use sods_causal::CausalEventRecorder;

    let mut recorder = CausalEventRecorder::new();

    // Record first event
    recorder
        .record_event(
            CausalEvent::builder()
                .agent_id(test_address())
                .nonce(0)
                .sequence_index(0)
                .build()
                .unwrap(),
        )
        .unwrap();

    // Try to skip sequence index (should fail)
    let result = recorder.record_event(
        CausalEvent::builder()
            .agent_id(test_address())
            .nonce(0)
            .sequence_index(5) // Gap!
            .build()
            .unwrap(),
    );

    match result {
        Err(CausalError::SequenceGap { expected, actual }) => {
            assert_eq!(expected, 1);
            assert_eq!(actual, 5);
        }
        _ => panic!("Expected SequenceGap error"),
    }
}

/// Test that invalid result values are rejected.
#[test]
fn test_invalid_result_rejection() {
    // Test all valid results work
    for result in VALID_RESULTS {
        let event = CausalEvent::builder()
            .agent_id(test_address())
            .result(*result)
            .build();
        assert!(event.is_ok(), "Valid result '{}' should be accepted", result);
    }

    // Test invalid results are rejected
    let invalid_results = ["ok", "error", "pending", "complete", "SUCCESS", ""];
    for invalid in invalid_results {
        let result = CausalEvent::builder()
            .agent_id(test_address())
            .result(invalid)
            .build();

        match result {
            Err(CausalError::InvalidResult(msg)) => {
                assert_eq!(msg, invalid);
            }
            Ok(_) => panic!("Invalid result '{}' should be rejected", invalid),
            Err(e) => panic!("Unexpected error for '{}': {:?}", invalid, e),
        }
    }
}

/// Test that events from different agents don't interfere with each other.
#[test]
fn test_multiple_agents_isolation() {
    use sods_causal::CausalEventRecorder;

    let mut recorder = CausalEventRecorder::new();

    let agent1: Address = "0x1111111111111111111111111111111111111111"
        .parse()
        .unwrap();
    let agent2: Address = "0x2222222222222222222222222222222222222222"
        .parse()
        .unwrap();

    // Agent 1: record events at nonce 0, 1, 2
    for nonce in 0..3u64 {
        recorder
            .record_event(
                CausalEvent::builder()
                    .agent_id(agent1)
                    .nonce(nonce)
                    .sequence_index(0)
                    .event_type("agent1_event")
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }

    // Agent 2: record events at nonce 0, 1 (independent sequence)
    for nonce in 0..2u64 {
        recorder
            .record_event(
                CausalEvent::builder()
                    .agent_id(agent2)
                    .nonce(nonce)
                    .sequence_index(0)
                    .event_type("agent2_event")
                    .build()
                    .unwrap(),
            )
            .unwrap();
    }

    // Verify isolation
    assert_eq!(recorder.agent_count(), 2);
    assert_eq!(recorder.get_agent_events(&agent1).unwrap().len(), 3);
    assert_eq!(recorder.get_agent_events(&agent2).unwrap().len(), 2);

    // Verify event types are correct
    for event in recorder.get_agent_events(&agent1).unwrap() {
        assert_eq!(event.event_type, "agent1_event");
    }
    for event in recorder.get_agent_events(&agent2).unwrap() {
        assert_eq!(event.event_type, "agent2_event");
    }
}
