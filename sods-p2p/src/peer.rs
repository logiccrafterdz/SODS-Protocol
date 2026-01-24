//! SODS Peer - server side node that serves proofs.

use futures::StreamExt;
use k256::ecdsa::SigningKey;
use libp2p::{
    identify,
    identity::Keypair,
    request_response, gossipsub,
    swarm::{Swarm, SwarmEvent},
    Multiaddr, PeerId,
};
use rand::rngs::OsRng;
use tracing::{debug, info, warn};
use tokio::sync::broadcast;

use sods_core::BehavioralMerkleTree;
use sods_verifier::BlockVerifier;

use crate::behavior::{SodsBehaviour, SodsBehaviourEvent};
use crate::cache::{BlockCache, CachedBlock};
use crate::error::{Result, SodsP2pError};
use crate::protocol::{ProofRequest, ProofResponse};
use crate::threats::{ThreatRule, THREATS_TOPIC};

/// A SODS peer that serves behavioral proofs to the network.
pub struct SodsPeer {
    swarm: Swarm<SodsBehaviour>,
    verifier: BlockVerifier,
    cache: BlockCache,
    local_peer_id: PeerId,
    signing_key: SigningKey,
    threat_tx: broadcast::Sender<ThreatRule>,
}

impl SodsPeer {
    /// Create a new SODS peer.
    ///
    /// # Arguments
    ///
    /// * `rpc_url` - RPC endpoint for fetching block data
    pub fn new(rpc_url: &str) -> Result<Self> {
        let verifier = BlockVerifier::new(&[rpc_url.to_string()])?;
        let keypair = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(keypair.public());

        // Generate secp256k1 signing key for message signing
        let signing_key = SigningKey::random(&mut OsRng);

        // Threat broadcast channel (capacity 100)
        let (threat_tx, _) = broadcast::channel(100);

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .map_err(|e| SodsP2pError::NetworkError(format!("TCP error: {}", e)))?
            .with_quic()
            .with_other_transport(|key| {
                libp2p_webrtc::tokio::Transport::new(
                    key.clone(),
                    libp2p_webrtc::tokio::Certificate::generate(&mut rand::thread_rng())?,
                )
            })
            .map_err(|e| SodsP2pError::NetworkError(format!("WebRTC error: {}", e)))?
            .with_behaviour(|_key| SodsBehaviour::new(&keypair))
            .map_err(|e| SodsP2pError::NetworkError(format!("Behaviour error: {}", e)))?
            .build();
        
        // Subscribe to threats topic
        let topic = gossipsub::IdentTopic::new(THREATS_TOPIC);
        swarm.behaviour_mut().gossipsub.subscribe(&topic)
            .map_err(|e| SodsP2pError::NetworkError(format!("Subscription error: {:?}", e)))?;

        info!("Created SODS peer with ID: {}", local_peer_id);

        Ok(Self {
            swarm,
            verifier,
            cache: BlockCache::new(),
            local_peer_id,
            signing_key,
            threat_tx,
        })
    }

    /// Get the local peer ID.
    pub fn peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Publish a threat rule to the network.
    pub fn publish_threat(&mut self, rule: &ThreatRule) -> Result<()> {
        let topic = gossipsub::IdentTopic::new(THREATS_TOPIC);
        let data = serde_json::to_vec(rule)
            .map_err(|e| SodsP2pError::SerializationError(e.to_string()))?;
        
        self.swarm.behaviour_mut().gossipsub.publish(topic, data)
            .map_err(|e| SodsP2pError::NetworkError(format!("Publish error: {:?}", e)))?;
        
        info!("Published threat rule: {}", rule.id);
        Ok(())
    }

    /// Subscribe to incoming threat rules.
    pub fn subscribe_threats(&self) -> broadcast::Receiver<ThreatRule> {
        self.threat_tx.subscribe()
    }

    /// Connect to bootstrap nodes.
    pub async fn connect_bootstrap(&mut self, addrs: &[Multiaddr]) -> Result<()> {
        for addr in addrs {
            info!("Dialing bootstrap node: {}", addr);
            self.swarm
                .dial(addr.clone())
                .map_err(|e| SodsP2pError::NetworkError(format!("Dial error: {}", e)))?;
        }
        Ok(())
    }

    /// Start listening on the specified address.
    ///
    /// # Arguments
    ///
    /// * `addr` - Multiaddr to listen on (e.g., "/ip4/0.0.0.0/tcp/0")
    pub async fn listen(&mut self, addr: &str) -> Result<()> {
        let addr: Multiaddr = addr
            .parse()
            .map_err(|e| SodsP2pError::NetworkError(format!("Invalid address: {}", e)))?;

        self.swarm
            .listen_on(addr)
            .map_err(|e| SodsP2pError::NetworkError(format!("Listen error: {}", e)))?;

        info!("Peer {} listening...", self.local_peer_id);

        // Event loop
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {}", address);
                }
                SwarmEvent::Behaviour(SodsBehaviourEvent::Identify(event)) => {
                    self.handle_identify_event(event);
                }
                SwarmEvent::Behaviour(SodsBehaviourEvent::RequestResponse(event)) => {
                    self.handle_request_response_event(event).await;
                }
                SwarmEvent::Behaviour(SodsBehaviourEvent::Puzzle(event)) => {
                    self.handle_puzzle_event(event).await;
                }
                SwarmEvent::Behaviour(SodsBehaviourEvent::Gossipsub(event)) => {
                    self.handle_gossip_event(event);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    debug!("Connected to peer: {}", peer_id);
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    debug!("Disconnected from peer: {}", peer_id);
                }
                _ => {}
            }
        }
    }

    /// Handle gossipsub events.
    fn handle_gossip_event(&mut self, event: gossipsub::Event) {
        match event {
            gossipsub::Event::Message { propagation_source, message_id: _, message } => {
                if let Ok(rule) = serde_json::from_slice::<ThreatRule>(&message.data) {
                    info!("Received threat rule '{}' from {}", rule.id, propagation_source);
                    
                    // Validate rule
                    if rule.verify() {
                         // Forward to local listeners
                         let _ = self.threat_tx.send(rule);
                    } else {
                        warn!("Received INVALID threat rule from {}", propagation_source);
                    }
                } else {
                    warn!("Received malformed gossip message");
                }
            }
            _ => {}
        }
    }

    /// Handle identify protocol events.
    fn handle_identify_event(&mut self, event: identify::Event) {
        match event {
            identify::Event::Received { peer_id, info, .. } => {
                debug!(
                    "Identified peer {} running {} with {} addresses",
                    peer_id,
                    info.agent_version,
                    info.listen_addrs.len()
                );
                // Add peer addresses
                for addr in info.listen_addrs {
                    self.swarm.add_peer_address(peer_id, addr);
                }
            }
            identify::Event::Sent { peer_id, .. } => {
                debug!("Sent identify info to {}", peer_id);
            }
            identify::Event::Pushed { peer_id, .. } => {
                debug!("Pushed identify info to {}", peer_id);
            }
            identify::Event::Error { peer_id, error, .. } => {
                warn!("Identify error with {}: {:?}", peer_id, error);
            }
        }
    }

    /// Handle request-response events.
    async fn handle_request_response_event(
        &mut self,
        event: request_response::Event<ProofRequest, ProofResponse>,
    ) {
        use request_response::Event;

        match event {
            Event::Message { peer, message } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    info!("Received request from {}: {:?}", peer, request);
                    let response = self.handle_proof_request(request).await;

                    if let Err(e) = self
                        .swarm
                        .behaviour_mut()
                        .request_response
                        .send_response(channel, response)
                    {
                        warn!("Failed to send response: {:?}", e);
                    }
                }
                request_response::Message::Response { .. } => {
                    // Peers don't receive responses
                }
            },
            Event::OutboundFailure { peer, error, .. } => {
                warn!("Outbound failure to {}: {:?}", peer, error);
            }
            Event::InboundFailure { peer, error, .. } => {
                warn!("Inbound failure from {}: {:?}", peer, error);
            }
            Event::ResponseSent { peer, .. } => {
                debug!("Response sent to {}", peer);
            }
        }
    }

    /// Handle an incoming proof request.
    async fn handle_proof_request(&mut self, request: ProofRequest) -> ProofResponse {
        let ProofRequest {
            symbol,
            block_number,
        } = request;

        // Check cache first (clone to avoid borrow issues)
        if let Some(cached) = self.cache.get(block_number).cloned() {
            debug!("Cache hit for block {}", block_number);
            return self.generate_proof_from_cache(&cached, &symbol);
        }

        // Fetch and verify via Layer 1
        match self
            .verifier
            .verify_symbol_in_block(&symbol, block_number)
            .await
        {
            Ok(result) => {
                if result.is_verified {
                    let root = result
                        .merkle_root
                        .map(|v| {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&v);
                            arr
                        })
                        .unwrap_or([0u8; 32]);

                    ProofResponse::success_signed(vec![], root, result.occurrences, &self.signing_key)
                } else {
                    ProofResponse::error_signed(
                        result.error.unwrap_or_else(|| "Symbol not found".into()),
                        &self.signing_key,
                    )
                }
            }
            Err(e) => ProofResponse::error_signed(e.to_string(), &self.signing_key),
        }
    }

    /// Generate proof from cached block data.
    fn generate_proof_from_cache(&self, cached: &CachedBlock, symbol: &str) -> ProofResponse {
        if !cached.has_symbol(symbol) {
            return ProofResponse::error_signed(
                format!("Symbol '{}' not in cached block", symbol),
                &self.signing_key,
            );
        }

        let occurrences = cached.count_symbol(symbol);
        let bmt = BehavioralMerkleTree::new(cached.symbols.clone());

        if let Some(sym) = cached.symbols.iter().find(|s| s.symbol() == symbol) {
            if let Some(proof) = bmt.generate_proof(symbol, sym.log_index()) {
                let proof_bytes = proof.serialize();
                return ProofResponse::success_signed(
                    proof_bytes,
                    cached.bmt_root,
                    occurrences,
                    &self.signing_key,
                );
            }
        }

        ProofResponse::error_signed("Failed to generate proof", &self.signing_key)
    }
    /// Handle puzzle-related events.
    async fn handle_puzzle_event(
        &mut self,
        event: request_response::Event<crate::protocol::PuzzleChallenge, crate::protocol::PuzzleSolution>,
    ) {
        use request_response::Event;

        match event {
            Event::Message { peer, message } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    info!("Received PoB challenge from {}: {:?}", peer, request);
                    let response = self.solve_puzzle(request).await;

                    if let Err(e) = self
                        .swarm
                        .behaviour_mut()
                        .puzzle
                        .send_response(channel, response)
                    {
                        warn!("Failed to send puzzle response: {:?}", e);
                    }
                }
                request_response::Message::Response { .. } => {
                    // Peers don't receive puzzle responses
                }
            },
            _ => {
                // Handle failures if necessary
            }
        }
    }

    /// Solve a Proof-of-Behavior puzzle.
    async fn solve_puzzle(&mut self, challenge: crate::protocol::PuzzleChallenge) -> crate::protocol::PuzzleSolution {
        match self
            .verifier
            .verify_symbol_in_block(&challenge.symbol, challenge.block_number)
            .await
        {
            Ok(result) => {
                crate::protocol::PuzzleSolution {
                    occurrences: result.occurrences as u32,
                    success: true,
                }
            }
            Err(_) => crate::protocol::PuzzleSolution {
                occurrences: 0,
                success: false,
            },
        }
    }
}
