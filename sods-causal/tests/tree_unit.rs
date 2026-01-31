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
fn test_tree_root_consistency() {
    let events = vec![
        create_event(0, 0),
        create_event(0, 1),
        create_event(1, 0),
    ];

    let tree1 = CausalMerkleTree::new(events.clone()).unwrap();
    let tree2 = CausalMerkleTree::new(events).unwrap();

    assert_eq!(tree1.root, tree2.root);
}

#[test]
fn test_causal_ordering_enforced() {
    let events = vec![
        create_event(1, 0),
        create_event(0, 0), // Out of order
    ];

    let result = CausalMerkleTree::new(events);
    assert!(result.is_err());
}

#[test]
fn test_single_event_tree() {
    let event = create_event(0, 0);
    let tree = CausalMerkleTree::new(vec![event]).unwrap();
    
    assert_ne!(tree.root, ethers::types::H256::zero());
    assert_eq!(tree.levels_len(), 1);
    assert_eq!(tree.get_hash(0, 0).unwrap(), tree.root);
}

#[test]
fn test_empty_tree() {
    let tree = CausalMerkleTree::new(vec![]).unwrap();
    assert_eq!(tree.root, ethers::types::H256::zero());
    assert!(tree.events().is_empty());
}
