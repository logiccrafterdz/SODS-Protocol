//! Libp2p network behavior combining mDNS and request-response.

use libp2p::{
    mdns, request_response,
    swarm::NetworkBehaviour,
    StreamProtocol,
};

use crate::protocol::{ProofRequest, ProofResponse};

/// Protocol identifier for SODS.
pub fn sods_protocol() -> StreamProtocol {
    StreamProtocol::new("/sods/proof/1.0.0")
}

/// Combined network behavior for SODS P2P.
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "SodsBehaviourEvent")]
pub struct SodsBehaviour {
    /// mDNS for local peer discovery
    pub mdns: mdns::tokio::Behaviour,
    /// Request-response for proof exchange
    pub request_response: request_response::cbor::Behaviour<ProofRequest, ProofResponse>,
}

/// Events emitted by the SODS behavior.
#[derive(Debug)]
pub enum SodsBehaviourEvent {
    /// mDNS discovery event
    Mdns(mdns::Event),
    /// Request-response event
    RequestResponse(request_response::Event<ProofRequest, ProofResponse>),
}

impl From<mdns::Event> for SodsBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        SodsBehaviourEvent::Mdns(event)
    }
}

impl From<request_response::Event<ProofRequest, ProofResponse>> for SodsBehaviourEvent {
    fn from(event: request_response::Event<ProofRequest, ProofResponse>) -> Self {
        SodsBehaviourEvent::RequestResponse(event)
    }
}

impl SodsBehaviour {
    /// Create a new SODS behavior with the given peer ID.
    pub fn new(local_peer_id: libp2p::PeerId) -> Self {
        // mDNS config
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            local_peer_id,
        )
        .expect("Failed to create mDNS behaviour");

        // Request-response config using CBOR codec
        let request_response = request_response::cbor::Behaviour::new(
            [(sods_protocol(), request_response::ProtocolSupport::Full)],
            request_response::Config::default(),
        );

        Self {
            mdns,
            request_response,
        }
    }
}
