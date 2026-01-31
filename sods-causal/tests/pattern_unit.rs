use std::time::Duration;
use ethers::types::Address;
use sods_causal::{CausalEvent, AgentBehaviorPattern};

fn create_event(timestamp: u64, result: &str) -> CausalEvent {
    CausalEvent::builder()
        .agent_id(Address::random())
        .nonce(0)
        .sequence_index(0)
        .event_type("task_executed")
        .result(result)
        .timestamp(timestamp)
        .build()
        .unwrap()
}

#[test]
fn test_exact_count_pattern() {
    let events = vec![
        create_event(100, "success"),
        create_event(200, "success"),
        create_event(300, "success"),
    ];

    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 3,
        max_count: Some(3),
        time_window: None,
    };

    let matches = pattern.matches(&events, 500);
    assert_eq!(matches.len(), 3);
}

#[test]
fn test_range_count_pattern() {
    let events = vec![
        create_event(100, "success"),
        create_event(200, "success"),
        create_event(300, "success"),
        create_event(400, "success"),
    ];

    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 2,
        max_count: Some(10),
        time_window: None,
    };

    let matches = pattern.matches(&events, 500);
    assert_eq!(matches.len(), 4);
}

#[test]
fn test_time_window_filter() {
    let events = vec![
        create_event(100, "success"), // Expired
        create_event(450, "success"), // Within window
        create_event(480, "success"), // Within window
    ];

    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "success".to_string(),
        min_count: 1,
        max_count: None,
        time_window: Some(Duration::from_secs(100)),
    };

    let matches = pattern.matches(&events, 500);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].timestamp, 450);
}

#[test]
fn test_result_filter() {
    let events = vec![
        create_event(100, "success"),
        create_event(200, "failure"),
        create_event(300, "success"),
    ];

    let pattern = AgentBehaviorPattern {
        event_type: "task_executed".to_string(),
        result_filter: "failure".to_string(),
        min_count: 1,
        max_count: None,
        time_window: None,
    };

    let matches = pattern.matches(&events, 500);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].result, "failure");
}
