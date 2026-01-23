//! SODS Client - requests proofs from the P2P network.

use futures::StreamExt;
use libp2p::{
    identify,
    identity::Keypair,
    request_response::{self, OutboundRequestId},
    swarm::{Swarm, SwarmEvent},
    Multiaddr, PeerId,
};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, info, warn};

use sods_verifier::BlockVerifier;

use crate::behavior::{SodsBehaviour, SodsBehaviourEvent};
use crate::consensus::{evaluate_consensus, DEFAULT_THRESHOLD};
use crate::error::{Result, SodsP2pError};
use crate::protocol::{ProofRequest, ProofResponse};
use crate::reputation::ReputationTracker;

/// Number of peers to query for consensus.
const QUERY_PEER_COUNT: usize = 3;

/// Timeout for P2P requests.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Discovery timeout before querying.
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

/// Result of P2P verification.
#[derive(Debug, Clone)]
pub struct P2pVerificationResult {
    /// Whether the symbol was verified.
    pub is_verified: bool,
    /// Number of agreeing peers.
    pub agreeing_peers: usize,
    /// Total peers queried.
    pub total_peers: usize,
    /// BMT root (if verified).
    pub bmt_root: Option<[u8; 32]>,
    /// Whether fallback to RPC was used.
    pub used_fallback: bool,
    /// Symbol that was verified.
    pub symbol: String,
    /// Block number that was queried.
    pub block_number: u64,
}

/// A SODS client that requests proofs via P2P.
pub struct SodsClient {
    swarm: Swarm<SodsBehaviour>,
    reputation: ReputationTracker,
    fallback_verifier: Option<BlockVerifier>,
    known_peers: HashSet<PeerId>,
    local_peer_id: PeerId,
    pending_requests: HashMap<OutboundRequestId, PeerId>,
}

impl SodsClient {
    /// Create a new SODS client (P2P only).
    pub fn new() -> Result<Self> {
        Self::build(None)
    }

    /// Create a new SODS client with RPC fallback.
    pub fn with_fallback(rpc_url: &str) -> Result<Self> {
        let verifier = BlockVerifier::new(&[rpc_url.to_string()])?;
        Self::build(Some(verifier))
    }

    fn build(fallback_verifier: Option<BlockVerifier>) -> Result<Self> {
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| SodsP2pError::NetworkError(format!("TCP error: {}", e)))?
            .with_behaviour(|_key| SodsBehaviour::new(&keypair))
            .map_err(|e| SodsP2pError::NetworkError(format!("Behaviour error: {}", e)))?
            .build();

        info!("Created SODS client with ID: {}", local_peer_id);

        Ok(Self {
            swarm,
            reputation: ReputationTracker::new(),
            fallback_verifier,
            known_peers: HashSet::new(),
            local_peer_id,
            pending_requests: HashMap::new(),
        })
    }

    /// Get the local peer ID.
    pub fn peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Connect to bootstrap nodes and discover peers.
    pub async fn connect_bootstrap(&mut self, addrs: &[Multiaddr]) -> Result<()> {
        // Start listening first
        let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0".parse().unwrap();
        self.swarm
            .listen_on(listen_addr)
            .map_err(|e| SodsP2pError::NetworkError(format!("Listen error: {}", e)))?;

        // Dial bootstrap nodes
        for addr in addrs {
            info!("Dialing bootstrap node: {}", addr);
            self.swarm
                .dial(addr.clone())
                .map_err(|e| SodsP2pError::NetworkError(format!("Dial error: {}", e)))?;
        }

        // Wait for peer discovery
        let discovery = timeout(DISCOVERY_TIMEOUT, async {
            while self.known_peers.len() < QUERY_PEER_COUNT {
                match self.swarm.select_next_some().await {
                    SwarmEvent::Behaviour(SodsBehaviourEvent::Identify(event)) => {
                        self.handle_identify_event(event);
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        debug!("Listening on {}", address);
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        debug!("Connected to {}", peer_id);
                    }
                    _ => {}
                }
            }
        });

        let _ = discovery.await;

        info!("Discovered {} peers", self.known_peers.len());
        Ok(())
    }

    /// Handle identify events.
    fn handle_identify_event(&mut self, event: identify::Event) {
        match event {
            identify::Event::Received { peer_id, info, .. } => {
                if peer_id != self.local_peer_id {
                    debug!("Identified peer: {} ({})", peer_id, info.agent_version);
                    self.known_peers.insert(peer_id);
                    for addr in info.listen_addrs {
                        self.swarm.add_peer_address(peer_id, addr);
                    }
                }
            }
            _ => {}
        }
    }

    /// Verify a symbol via P2P network.
    pub async fn verify_via_p2p(
        &mut self,
        symbol: &str,
        block_number: u64,
    ) -> Result<P2pVerificationResult> {
        if self.known_peers.is_empty() {
            return self.try_fallback(symbol, block_number).await;
        }

        // Select peers to query
        let peers_list: Vec<_> = self.known_peers.iter().cloned().collect();
        let selected = self
            .reputation
            .select_best_peers(&peers_list, QUERY_PEER_COUNT);

        if selected.is_empty() {
            return self.try_fallback(symbol, block_number).await;
        }

        // Send requests
        let request = ProofRequest {
            symbol: symbol.to_string(),
            block_number,
        };

        for peer_id in &selected {
            let request_id = self
                .swarm
                .behaviour_mut()
                .request_response
                .send_request(peer_id, request.clone());
            self.pending_requests.insert(request_id, *peer_id);
        }

        // Collect responses
        let responses = self.collect_responses(selected.len()).await;

        if responses.is_empty() {
            return self.try_fallback(symbol, block_number).await;
        }

        // Verify signatures and filter invalid responses
        let valid_responses: Vec<_> = responses
            .into_iter()
            .filter(|(peer_id, resp)| {
                if resp.is_signed() && resp.verify_signature() {
                    true
                } else {
                    warn!("Invalid signature from peer {}", peer_id);
                    self.reputation.penalize(peer_id);
                    false
                }
            })
            .collect();

        if valid_responses.is_empty() {
            return self.try_fallback(symbol, block_number).await;
        }

        // Evaluate consensus
        let consensus = evaluate_consensus(valid_responses.clone(), &self.reputation, DEFAULT_THRESHOLD);

        // Update reputation
        for (peer_id, _) in &valid_responses {
            if consensus.conflicting_peers.contains(peer_id) {
                self.reputation.penalize(peer_id);
            } else {
                self.reputation.reward(peer_id);
            }
        }

        if consensus.is_verified {
            Ok(P2pVerificationResult {
                is_verified: true,
                agreeing_peers: consensus.agreeing_peers,
                total_peers: consensus.total_peers,
                bmt_root: consensus.bmt_root,
                used_fallback: false,
                symbol: symbol.to_string(),
                block_number,
            })
        } else if self.fallback_verifier.is_some() {
            self.try_fallback(symbol, block_number).await
        } else {
            Err(SodsP2pError::ConsensusFailure {
                agreeing: consensus.agreeing_peers,
                total: consensus.total_peers,
            })
        }
    }

    /// Collect responses from pending requests.
    async fn collect_responses(&mut self, expected_count: usize) -> Vec<(PeerId, ProofResponse)> {
        let mut responses = Vec::new();

        let collection = timeout(REQUEST_TIMEOUT, async {
            while responses.len() < expected_count {
                match self.swarm.select_next_some().await {
                    SwarmEvent::Behaviour(SodsBehaviourEvent::RequestResponse(event)) => {
                        if let request_response::Event::Message {
                            peer,
                            message:
                                request_response::Message::Response {
                                    response,
                                    request_id,
                                    ..
                                },
                        } = event
                        {
                            self.pending_requests.remove(&request_id);
                            responses.push((peer, response));
                        }
                    }
                    SwarmEvent::Behaviour(SodsBehaviourEvent::Identify(event)) => {
                        self.handle_identify_event(event);
                    }
                    _ => {}
                }
            }
        });

        let _ = collection.await;
        responses
    }

    /// Try fallback to RPC verification.
    async fn try_fallback(
        &mut self,
        symbol: &str,
        block_number: u64,
    ) -> Result<P2pVerificationResult> {
        let verifier = self
            .fallback_verifier
            .as_ref()
            .ok_or(SodsP2pError::NoAvailablePeers)?;

        info!("Using RPC fallback for verification");

        let result = verifier.verify_symbol_in_block(symbol, block_number).await?;

        let bmt_root = result.merkle_root.map(|v| {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&v);
            arr
        });

        Ok(P2pVerificationResult {
            is_verified: result.is_verified,
            agreeing_peers: 0,
            total_peers: 0,
            bmt_root,
            used_fallback: true,
            symbol: symbol.to_string(),
            block_number,
        })
    }

    /// Get the reputation tracker.
    pub fn reputation(&self) -> &ReputationTracker {
        &self.reputation
    }

    /// Get the number of known peers.
    pub fn known_peer_count(&self) -> usize {
        self.known_peers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SodsClient::new();
        assert!(client.is_ok());
    }
}
