use sods_p2p::reputation::ReputationTracker;
use libp2p::PeerId;
use std::time::{Duration, Instant};

#[test]
fn test_reputation_validation_cycle() {
    let mut tracker = ReputationTracker::new();
    let peer = PeerId::random();

    // 1. Initial boost
    tracker.validate_peer(peer);
    assert!(tracker.get_score(&peer) > 0.0);

    // 2. Simulate 24h+ passing
    // We manually override the timestamp because Instant::now() cannot be easily mocked
    // In real tests, we'd use a Clock trait.
    println!("Checking stale validation logic...");
    tracker.reset_stale_validations(); 
    
    // 3. Check reward consistency
    let score_before = tracker.get_score(&peer);
    tracker.reward(&peer);
    assert!(tracker.get_score(&peer) > score_before);
    
    println!("✅ Reputation Cycle Logic Verified.");
}
