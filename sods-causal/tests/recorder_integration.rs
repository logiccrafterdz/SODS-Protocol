//! Integration tests for CausalEventRecorder.
//!
//! These tests verify the recorder's ability to manage events
//! across multiple agents with proper causal ordering enforcement.

use ethers::types::{Address, H256};
use sods_causal::{CausalEvent, CausalEventRecorder};

fn agent_a() -> Address {
    "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        .parse()
        .unwrap()
}

fn agent_b() -> Address {
    "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        .parse()
        .unwrap()
}

/// Test recording multiple events for the same agent with proper ordering.
#[test]
fn test_record_multiple_events_same_agent() {
    let mut recorder = CausalEventRecorder::new();

    // Record a sequence of events within the same transaction (nonce=0)
    for seq in 0..5u32 {
        let event = CausalEvent::builder()
            .agent_id(agent_a())
            .nonce(0)
            .sequence_index(seq)
            .event_type(format!("step_{}", seq))
            .result("success")
            .timestamp(1700000000 + seq as u64)
            .build()
            .unwrap();

        recorder.record_event(event).unwrap();
    }

    // Move to next transaction (nonce=1)
    for seq in 0..3u32 {
        let event = CausalEvent::builder()
            .agent_id(agent_a())
            .nonce(1)
            .sequence_index(seq)
            .event_type(format!("tx2_step_{}", seq))
            .result("success")
            .timestamp(1700000100 + seq as u64)
            .build()
            .unwrap();

        recorder.record_event(event).unwrap();
    }

    let events = recorder.get_agent_events(&agent_a()).unwrap();
    assert_eq!(events.len(), 8); // 5 + 3

    // Verify ordering is preserved
    for i in 0..events.len() - 1 {
        assert!(events[i] < events[i + 1], "Events should be causally ordered");
    }
}

/// Test recording events for multiple independent agents.
#[test]
fn test_record_events_multiple_agents() {
    let mut recorder = CausalEventRecorder::new();

    // Interleave events from two agents
    recorder
        .record_event(
            CausalEvent::builder()
                .agent_id(agent_a())
                .nonce(0)
                .sequence_index(0)
                .event_type("A_init")
                .build()
                .unwrap(),
        )
        .unwrap();

    recorder
        .record_event(
            CausalEvent::builder()
                .agent_id(agent_b())
                .nonce(0)
                .sequence_index(0)
                .event_type("B_init")
                .build()
                .unwrap(),
        )
        .unwrap();

    recorder
        .record_event(
            CausalEvent::builder()
                .agent_id(agent_a())
                .nonce(0)
                .sequence_index(1)
                .event_type("A_step1")
                .build()
                .unwrap(),
        )
        .unwrap();

    recorder
        .record_event(
            CausalEvent::builder()
                .agent_id(agent_b())
                .nonce(1)
                .sequence_index(0)
                .event_type("B_step1")
                .build()
                .unwrap(),
        )
        .unwrap();

    assert_eq!(recorder.agent_count(), 2);
    assert_eq!(recorder.get_agent_events(&agent_a()).unwrap().len(), 2);
    assert_eq!(recorder.get_agent_events(&agent_b()).unwrap().len(), 2);
}

/// Test that causal ordering is strictly enforced.
#[test]
fn test_causal_ordering_enforcement() {
    let mut recorder = CausalEventRecorder::new();

    // Valid: nonce=0, seq=0
    recorder
        .record_event(
            CausalEvent::builder()
                .agent_id(agent_a())
                .nonce(0)
                .sequence_index(0)
                .build()
                .unwrap(),
        )
        .unwrap();

    // Invalid: nonce=0, seq=2 (gap from seq=0 to seq=2)
    let gap_result = recorder.record_event(
        CausalEvent::builder()
            .agent_id(agent_a())
            .nonce(0)
            .sequence_index(2)
            .build()
            .unwrap(),
    );
    assert!(gap_result.is_err());

    // Valid: nonce=0, seq=1
    recorder
        .record_event(
            CausalEvent::builder()
                .agent_id(agent_a())
                .nonce(0)
                .sequence_index(1)
                .build()
                .unwrap(),
        )
        .unwrap();

    // Invalid: nonce=3 (gap from nonce=0 to nonce=3)
    let nonce_gap = recorder.record_event(
        CausalEvent::builder()
            .agent_id(agent_a())
            .nonce(3)
            .sequence_index(0)
            .build()
            .unwrap(),
    );
    assert!(nonce_gap.is_err());

    // Valid: nonce=1, seq=0
    recorder
        .record_event(
            CausalEvent::builder()
                .agent_id(agent_a())
                .nonce(1)
                .sequence_index(0)
                .build()
                .unwrap(),
        )
        .unwrap();

    assert_eq!(recorder.get_agent_events(&agent_a()).unwrap().len(), 3);
}

/// Test that metadata_hash is properly handled as optional.
#[test]
fn test_metadata_hash_optional_handling() {
    let mut recorder = CausalEventRecorder::new();

    // Event without metadata hash
    let event1 = CausalEvent::builder()
        .agent_id(agent_a())
        .nonce(0)
        .sequence_index(0)
        .event_type("no_metadata")
        .build()
        .unwrap();

    assert!(event1.metadata_hash.is_none());
    recorder.record_event(event1).unwrap();

    // Event with metadata hash
    let hash: H256 = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
        .parse()
        .unwrap();

    let event2 = CausalEvent::builder()
        .agent_id(agent_a())
        .nonce(0)
        .sequence_index(1)
        .event_type("with_metadata")
        .metadata_hash(hash)
        .build()
        .unwrap();

    assert!(event2.metadata_hash.is_some());
    assert_eq!(event2.metadata_hash.unwrap(), hash);
    recorder.record_event(event2).unwrap();

    // Verify both events stored correctly
    let events = recorder.get_agent_events(&agent_a()).unwrap();
    assert_eq!(events.len(), 2);
    assert!(events[0].metadata_hash.is_none());
    assert!(events[1].metadata_hash.is_some());

    // Test serialization with optional field
    let json1 = serde_json::to_string(&events[0]).unwrap();
    let json2 = serde_json::to_string(&events[1]).unwrap();

    assert!(json1.contains("\"metadata_hash\":null"));
    assert!(json2.contains("\"metadata_hash\":"));

    // Roundtrip
    let parsed1: CausalEvent = serde_json::from_str(&json1).unwrap();
    let parsed2: CausalEvent = serde_json::from_str(&json2).unwrap();

    assert_eq!(parsed1.metadata_hash, None);
    assert_eq!(parsed2.metadata_hash, Some(hash));
}
