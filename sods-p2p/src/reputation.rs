//! Peer reputation tracking.

use libp2p::PeerId;
use std::collections::HashMap;

/// Reward for consistent response.
const REWARD_AMOUNT: i32 = 1;

/// Penalty for conflicting response.
const PENALTY_AMOUNT: i32 = 2;

/// Minimum score before peer is considered unreliable.
const MIN_RELIABLE_SCORE: i32 = -5;

/// Tracks peer reliability based on response consistency.
#[derive(Debug, Clone, Default)]
pub struct ReputationTracker {
    scores: HashMap<PeerId, i32>,
}

impl ReputationTracker {
    /// Create a new reputation tracker.
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
        }
    }

    /// Reward a peer for consistent behavior.
    pub fn reward(&mut self, peer: &PeerId) {
        *self.scores.entry(*peer).or_insert(0) += REWARD_AMOUNT;
    }

    /// Penalize a peer for conflicting behavior.
    pub fn penalize(&mut self, peer: &PeerId) {
        *self.scores.entry(*peer).or_insert(0) -= PENALTY_AMOUNT;
    }

    /// Get a peer's current score.
    pub fn get_score(&self, peer: &PeerId) -> i32 {
        *self.scores.get(peer).unwrap_or(&0)
    }

    /// Check if a peer is considered reliable.
    pub fn is_reliable(&self, peer: &PeerId) -> bool {
        self.get_score(peer) >= MIN_RELIABLE_SCORE
    }

    /// Select the best peers by score.
    pub fn select_best_peers(&self, available: &[PeerId], count: usize) -> Vec<PeerId> {
        let mut peers: Vec<_> = available
            .iter()
            .filter(|p| self.is_reliable(p))
            .map(|p| (*p, self.get_score(p)))
            .collect();

        // Sort by score descending
        peers.sort_by(|a, b| b.1.cmp(&a.1));

        peers.into_iter().take(count).map(|(p, _)| p).collect()
    }

    /// Get list of unreliable peers (for blocking).
    pub fn get_unreliable_peers(&self) -> Vec<PeerId> {
        self.scores
            .iter()
            .filter(|(_, score)| **score < MIN_RELIABLE_SCORE)
            .map(|(peer, _)| *peer)
            .collect()
    }

    /// Number of tracked peers.
    pub fn len(&self) -> usize {
        self.scores.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn random_peer() -> PeerId {
        PeerId::random()
    }

    #[test]
    fn test_reward() {
        let mut tracker = ReputationTracker::new();
        let peer = random_peer();
        
        tracker.reward(&peer);
        assert_eq!(tracker.get_score(&peer), 1);
        
        tracker.reward(&peer);
        assert_eq!(tracker.get_score(&peer), 2);
    }

    #[test]
    fn test_penalize() {
        let mut tracker = ReputationTracker::new();
        let peer = random_peer();
        
        tracker.penalize(&peer);
        assert_eq!(tracker.get_score(&peer), -2);
    }

    #[test]
    fn test_reliability() {
        let mut tracker = ReputationTracker::new();
        let good_peer = random_peer();
        let bad_peer = random_peer();
        
        // Good peer
        tracker.reward(&good_peer);
        assert!(tracker.is_reliable(&good_peer));
        
        // Bad peer - penalize 3 times
        for _ in 0..3 {
            tracker.penalize(&bad_peer);
        }
        assert!(!tracker.is_reliable(&bad_peer));
    }

    #[test]
    fn test_select_best_peers() {
        let mut tracker = ReputationTracker::new();
        let peer1 = random_peer();
        let peer2 = random_peer();
        let peer3 = random_peer();
        
        tracker.reward(&peer1);
        tracker.reward(&peer1); // score: 2
        tracker.reward(&peer2); // score: 1
        // peer3: score 0
        
        let available = vec![peer1, peer2, peer3];
        let best = tracker.select_best_peers(&available, 2);
        
        assert_eq!(best.len(), 2);
        assert_eq!(best[0], peer1); // Highest score first
    }
}
