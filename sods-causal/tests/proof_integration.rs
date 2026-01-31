use ethers::types::Address;
use sods_causal::{CausalEvent, CausalMerkleTree};

fn create_event(nonce: u64, seq: u32) -> CausalEvent {
    CausalEvent::builder()
        .agent_id(Address::random())
        .nonce(nonce)
        .sequence_index(seq)
        .event_type("test")
        .result("success")
        .timestamp(1000)
        .build()
        .unwrap()
}

#[test]
fn test_generate_and_verify_proof() {
    let events = vec![
        create_event(0, 0),
        create_event(0, 1),
        create_event(1, 0),
        create_event(1, 1),
        create_event(2, 0),
    ];

    let tree = CausalMerkleTree::new(events).unwrap();

    for i in 0..tree.events().len() {
        let proof = tree.generate_proof(i);
        assert!(proof.verify(), "Proof for event {} should be valid", i);
    }
}

#[test]
fn test_tampered_event_fails() {
    let events = vec![create_event(0, 0), create_event(0, 1)];
    let tree = CausalMerkleTree::new(events).unwrap();
    let mut proof = tree.generate_proof(0);

    // Tamper with event data
    proof.event.result = "failure".to_string();

    assert!(!proof.verify());
}

#[test]
fn test_tampered_path_fails() {
    let events = vec![create_event(0, 0), create_event(0, 1)];
    let tree = CausalMerkleTree::new(events).unwrap();
    let mut proof = tree.generate_proof(0);

    // Tamper with merkle path
    if !proof.merkle_path.is_empty() {
        proof.merkle_path[0] = ethers::types::H256::random();
    }

    assert!(!proof.verify());
}

#[test]
fn test_wrong_root_fails() {
    let events = vec![create_event(0, 0)];
    let tree = CausalMerkleTree::new(events).unwrap();
    let mut proof = tree.generate_proof(0);

    // Change root
    proof.root = ethers::types::H256::random();

    assert!(!proof.verify());
}
