use sods_p2p::reputation::ReputationTracker;
use libp2p::PeerId;

#[test]
fn test_sybil_resistance_flow() {
    let mut tracker = ReputationTracker::new();
    let malicious_peer = PeerId::random();
    let honest_peer = PeerId::random();

    // 1. Both start at 0.0
    assert_eq!(tracker.get_score(&malicious_peer), 0.0);
    assert_eq!(tracker.get_score(&honest_peer), 0.0);

    // 2. Malicious peer fails puzzle (stays at 0.0 or penalized)
    tracker.penalize(&malicious_peer);
    assert_eq!(tracker.get_score(&malicious_peer), 0.0);
    assert!(!tracker.is_reliable(&malicious_peer));

    // 3. Honest peer solves puzzle (boosted to 0.5)
    for _ in 0..10 { tracker.reward(&honest_peer); }
    assert!(tracker.get_score(&honest_peer) >= 0.4);
    assert!(tracker.is_reliable(&honest_peer));

    // 4. Verification queries only select reliable peers
    let available = vec![malicious_peer, honest_peer];
    let selected = tracker.select_best_peers(&available, 1);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0], honest_peer);
}
