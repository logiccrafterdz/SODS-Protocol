//! Peer reputation tracking with decay.

use libp2p::PeerId;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Initial score for new peers.
const INITIAL_SCORE: f32 = 0.0;

/// Minimum score to be considered reliable.
const MIN_RELIABLE_SCORE: f32 = 0.4; // Must solve puzzle + 1 good response

/// Rate at which reputation decays (every DECAY_INTERVAL).
const DECAY_FACTOR: f32 = 0.95;

/// Tracks peer reliability based on response consistency.
#[derive(Debug, Clone)]
pub struct ReputationTracker {
    scores: HashMap<PeerId, f32>,
    last_validation: HashMap<PeerId, Instant>,
    last_decay: Instant,
}

impl Default for ReputationTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ReputationTracker {
    /// Create a new reputation tracker.
    pub fn new() -> Self {
        Self {
            scores: HashMap::new(),
            last_validation: HashMap::new(),
            last_decay: Instant::now(),
        }
    }

    /// Mark a peer as validated (solved a fresh puzzle).
    pub fn validate_peer(&mut self, peer: PeerId) {
        self.last_validation.insert(peer, Instant::now());
        // Boosting score as a reward for successful validation
        self.reward(&peer);
    }

    /// Reset reputation for peers that haven't been validated within the last 24h.
    pub fn reset_stale_validations(&mut self) {
        let now = Instant::now();
        let stale_threshold = Duration::from_secs(86400); // 24 hours

        for (peer, last_time) in self.last_validation.iter() {
            if now.duration_since(*last_time) > stale_threshold {
                if let Some(score) = self.scores.get_mut(peer) {
                    warn!("Resetting reputation for peer {} due to stale validation (24h+)", peer);
                    *score = INITIAL_SCORE;
                }
            }
        }
    }

    /// Apply decay if enough time has passed.
    pub fn decay_if_needed(&mut self) {
        if self.last_decay.elapsed() >= Duration::from_secs(60) {
            self.decay_all();
            self.last_decay = Instant::now();
        }
    }

    /// Decay all peer scores.
    pub fn decay_all(&mut self) {
        for score in self.scores.values_mut() {
            *score *= DECAY_FACTOR;
        }
    }

    /// Reward a peer for consistent behavior.
    /// score = min(score * 1.1 + 0.05, 1.0)
    pub fn reward(&mut self, peer: &PeerId) {
        let score = self.scores.entry(*peer).or_insert(INITIAL_SCORE);
        *score = (*score * 1.1 + 0.05).min(1.0);
    }

    /// Penalize a peer for conflicting behavior.
    /// score = max(score * 0.7 - 0.1, 0.0)
    pub fn penalize(&mut self, peer: &PeerId) {
        let score = self.scores.entry(*peer).or_insert(INITIAL_SCORE);
        *score = (*score * 0.7 - 0.1).max(0.0);
    }

    /// Get a peer's current score.
    pub fn get_score(&self, peer: &PeerId) -> f32 {
        *self.scores.get(peer).unwrap_or(&INITIAL_SCORE)
    }

    /// Check if a peer is considered reliable.
    pub fn is_reliable(&self, peer: &PeerId) -> bool {
        self.get_score(peer) >= MIN_RELIABLE_SCORE
    }

    /// Select the best peers by score (weighted probability could be better, but greedy is fine for PoC).
    pub fn select_best_peers(&self, available: &[PeerId], count: usize) -> Vec<PeerId> {
        let mut peers: Vec<_> = available
            .iter()
            .map(|p| (*p, self.get_score(p)))
            .collect();

        // Sort by score descending
        peers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        peers.into_iter().take(count).map(|(p, _)| p).collect()
    }

    /// Get list of unreliable peers (for blocking).
    pub fn get_unreliable_peers(&self) -> Vec<PeerId> {
        self.scores
            .iter()
            .filter(|(_, score)| **score < 0.05) // Very low score threshold for blocking
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
    fn test_reward_growth() {
        let mut tracker = ReputationTracker::new();
        let peer = random_peer();
        
        // Initial -> Reward
        tracker.reward(&peer);
        let s1 = tracker.get_score(&peer);
        assert!(s1 > INITIAL_SCORE);
        assert!(s1 <= 1.0);

        // Max cap
        for _ in 0..20 { tracker.reward(&peer); }
        assert_eq!(tracker.get_score(&peer), 1.0);
    }

    #[test]
    fn test_penalty_drop() {
        let mut tracker = ReputationTracker::new();
        let peer = random_peer();
        
        // Reward first to boost
        tracker.reward(&peer);
        let s1 = tracker.get_score(&peer);
        
        tracker.penalize(&peer);
        let s2 = tracker.get_score(&peer);
        assert!(s2 < s1);
        
        // Min floor
        for _ in 0..10 { tracker.penalize(&peer); }
        assert_eq!(tracker.get_score(&peer), 0.0);
    }

    #[test]
    fn test_pob_scoring_threshold() {
        let mut tracker = ReputationTracker::new();
        let peer = random_peer();
        
        // New peer = 0.0
        assert_eq!(tracker.get_score(&peer), 0.0);
        assert!(!tracker.is_reliable(&peer));

        // Solving puzzle (simulated by many rewards)
        for _ in 0..10 { tracker.reward(&peer); }
        
        // Should be reliable now
        assert!(tracker.get_score(&peer) >= MIN_RELIABLE_SCORE);
        assert!(tracker.is_reliable(&peer));
    }
}
