//! Social consensus logic for proof verification.

use libp2p::PeerId;
use std::collections::HashMap;

use crate::protocol::ProofResponse;
use crate::reputation::ReputationTracker;

/// Default consensus threshold (2/3 majority of reputation).
pub const DEFAULT_THRESHOLD: f64 = 0.66;

/// Result of consensus evaluation.
#[derive(Debug, Clone)]
pub struct ConsensusResult {
    /// Whether consensus was achieved.
    pub is_verified: bool,
    /// Number of peers that agreed on the result.
    pub agreeing_peers: usize,
    /// Total number of peers queried.
    pub total_peers: usize,
    /// The agreed-upon BMT root (if consensus reached).
    pub bmt_root: Option<[u8; 32]>,
    /// The agreed-upon proof bytes (if consensus reached).
    pub proof_bytes: Option<Vec<u8>>,
    /// Peers that provided conflicting responses.
    pub conflicting_peers: Vec<PeerId>,
}

impl ConsensusResult {
    /// Create a failed consensus result.
    pub fn failed(total_peers: usize) -> Self {
        Self {
            is_verified: false,
            agreeing_peers: 0,
            total_peers,
            bmt_root: None,
            proof_bytes: None,
            conflicting_peers: Vec::new(),
        }
    }
}

/// Evaluate consensus from peer responses using reputation weights.
///
/// Groups responses by BMT root and determines if enough WEIGHTED peers agree.
pub fn evaluate_consensus(
    responses: Vec<(PeerId, ProofResponse)>,
    reputation: &ReputationTracker,
    threshold: f64,
) -> ConsensusResult {
    if responses.is_empty() {
        return ConsensusResult::failed(0);
    }

    let total_count = responses.len();
    
    // Calculate total weight of all responding peers
    let total_weight: f32 = responses.iter()
        .map(|(p, _)| reputation.get_score(p))
        .sum();

    if total_weight <= 0.0 {
         // Edge case: all peers have 0 reputation? fallback to count?
         // For now, if 0 weight, we can't decide trusted consensus.
         return ConsensusResult::failed(total_count);
    }

    // Filter successful responses
    let successful: Vec<_> = responses
        .iter()
        .filter(|(_, r)| r.success)
        .collect();

    if successful.is_empty() {
        return ConsensusResult {
            is_verified: false,
            agreeing_peers: 0,
            total_peers: total_count,
            bmt_root: None,
            proof_bytes: None,
            conflicting_peers: responses.iter().map(|(p, _)| *p).collect(),
        };
    }

    // Group by BMT root
    let mut groups: HashMap<[u8; 32], Vec<(PeerId, &ProofResponse)>> = HashMap::new();
    for (peer, resp) in &successful {
        groups
            .entry(resp.bmt_root)
            .or_default()
            .push((*peer, resp));
    }

    // Find the group with highest weight
    let (largest_root, largest_group) = groups
        .into_iter()
        .max_by(|(_, g1), (_, g2)| {
            let w1: f32 = g1.iter().map(|(p, _)| reputation.get_score(p)).sum();
            let w2: f32 = g2.iter().map(|(p, _)| reputation.get_score(p)).sum();
            w1.partial_cmp(&w2).unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("successful is non-empty");

    let agreeing_weight: f32 = largest_group.iter().map(|(p, _)| reputation.get_score(p)).sum();
    let agreement_ratio = agreeing_weight / total_weight;

    // Identify conflicting peers (those not in the largest group)
    let agreeing_peers: Vec<_> = largest_group.iter().map(|(p, _)| *p).collect();
    let conflicting_peers: Vec<_> = responses
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| !agreeing_peers.contains(p))
        .collect();

    if agreement_ratio as f64 >= threshold {
        // Consensus reached
        let proof_bytes = largest_group.first().map(|(_, r)| r.proof_bytes.clone());
        
        ConsensusResult {
            is_verified: true,
            agreeing_peers: agreeing_peers.len(),
            total_peers: total_count,
            bmt_root: Some(largest_root),
            proof_bytes,
            conflicting_peers,
        }
    } else {
        // No consensus
        ConsensusResult {
            is_verified: false,
            agreeing_peers: agreeing_peers.len(),
            total_peers: total_count,
            bmt_root: None,
            proof_bytes: None,
            conflicting_peers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response(root: [u8; 32], success: bool) -> ProofResponse {
        if success {
            ProofResponse::success(vec![1, 2, 3], root, 1)
        } else {
            ProofResponse::error("failed")
        }
    }

    #[test]
    fn test_weighted_consensus() {
        let mut tracker = ReputationTracker::new();
        let peer1 = PeerId::random();
        let peer2 = PeerId::random(); // High rep
        let peer3 = PeerId::random(); // Malicious low rep

        // peer1: default 0.1
        // peer2: reward 10x -> high score
        for _ in 0..10 { tracker.reward(&peer2); } 
        // peer3: default 0.1

        let good_root = [0xAA; 32];
        let bad_root = [0xBB; 32];

        let responses = vec![
            (peer1, make_response(good_root, true)),
            (peer2, make_response(good_root, true)), // High weight agrees
            (peer3, make_response(bad_root, true)),  // Low weight disagrees
        ];

        let result = evaluate_consensus(responses, &tracker, DEFAULT_THRESHOLD);
        
        assert!(result.is_verified);
        assert_eq!(result.bmt_root, Some(good_root));
        assert!(result.conflicting_peers.contains(&peer3));
    }
}


