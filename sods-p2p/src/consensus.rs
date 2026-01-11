//! Social consensus logic for proof verification.

use libp2p::PeerId;
use std::collections::HashMap;

use crate::protocol::ProofResponse;

/// Default consensus threshold (2/3 majority).
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

/// Evaluate consensus from peer responses.
///
/// Groups responses by BMT root and determines if enough peers agree.
///
/// # Arguments
///
/// * `responses` - List of (peer_id, response) pairs
/// * `threshold` - Minimum fraction required for consensus (e.g., 0.67 for 2/3)
///
/// # Returns
///
/// ConsensusResult indicating whether consensus was reached.
pub fn evaluate_consensus(
    responses: Vec<(PeerId, ProofResponse)>,
    threshold: f64,
) -> ConsensusResult {
    if responses.is_empty() {
        return ConsensusResult::failed(0);
    }

    let total = responses.len();

    // Filter successful responses
    let successful: Vec<_> = responses
        .iter()
        .filter(|(_, r)| r.success)
        .collect();

    if successful.is_empty() {
        // All failed - no consensus possible
        return ConsensusResult {
            is_verified: false,
            agreeing_peers: 0,
            total_peers: total,
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

    // Find the largest agreeing group
    let (largest_root, largest_group) = groups
        .into_iter()
        .max_by_key(|(_, g)| g.len())
        .expect("successful is non-empty");

    let agreeing = largest_group.len();
    let agreement_ratio = agreeing as f64 / total as f64;

    // Identify conflicting peers (those not in the largest group)
    let agreeing_peers: Vec<_> = largest_group.iter().map(|(p, _)| *p).collect();
    let conflicting_peers: Vec<_> = responses
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| !agreeing_peers.contains(p))
        .collect();

    if agreement_ratio >= threshold {
        // Consensus reached
        let proof_bytes = largest_group.first().map(|(_, r)| r.proof_bytes.clone());
        
        ConsensusResult {
            is_verified: true,
            agreeing_peers: agreeing,
            total_peers: total,
            bmt_root: Some(largest_root),
            proof_bytes,
            conflicting_peers,
        }
    } else {
        // No consensus
        ConsensusResult {
            is_verified: false,
            agreeing_peers: agreeing,
            total_peers: total,
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
    fn test_unanimous_consensus() {
        let root = [0xAB; 32];
        let responses = vec![
            (PeerId::random(), make_response(root, true)),
            (PeerId::random(), make_response(root, true)),
            (PeerId::random(), make_response(root, true)),
        ];

        let result = evaluate_consensus(responses, DEFAULT_THRESHOLD);

        assert!(result.is_verified);
        assert_eq!(result.agreeing_peers, 3);
        assert_eq!(result.total_peers, 3);
        assert_eq!(result.bmt_root, Some(root));
        assert!(result.conflicting_peers.is_empty());
    }

    #[test]
    fn test_two_thirds_consensus() {
        let good_root = [0xAB; 32];
        let bad_root = [0xCD; 32];
        let bad_peer = PeerId::random();
        
        let responses = vec![
            (PeerId::random(), make_response(good_root, true)),
            (PeerId::random(), make_response(good_root, true)),
            (bad_peer, make_response(bad_root, true)),
        ];

        let result = evaluate_consensus(responses, DEFAULT_THRESHOLD);

        assert!(result.is_verified);
        assert_eq!(result.agreeing_peers, 2);
        assert_eq!(result.bmt_root, Some(good_root));
        assert_eq!(result.conflicting_peers.len(), 1);
        assert!(result.conflicting_peers.contains(&bad_peer));
    }

    #[test]
    fn test_no_consensus() {
        let responses = vec![
            (PeerId::random(), make_response([1; 32], true)),
            (PeerId::random(), make_response([2; 32], true)),
            (PeerId::random(), make_response([3; 32], true)),
        ];

        let result = evaluate_consensus(responses, DEFAULT_THRESHOLD);

        assert!(!result.is_verified);
        assert_eq!(result.agreeing_peers, 1);
        assert_eq!(result.total_peers, 3);
    }

    #[test]
    fn test_empty_responses() {
        let result = evaluate_consensus(vec![], DEFAULT_THRESHOLD);
        assert!(!result.is_verified);
        assert_eq!(result.total_peers, 0);
    }

    #[test]
    fn test_all_failed_responses() {
        let responses = vec![
            (PeerId::random(), make_response([0; 32], false)),
            (PeerId::random(), make_response([0; 32], false)),
        ];

        let result = evaluate_consensus(responses, DEFAULT_THRESHOLD);

        assert!(!result.is_verified);
        assert_eq!(result.conflicting_peers.len(), 2);
    }
}
