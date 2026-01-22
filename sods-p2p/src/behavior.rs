//! Libp2p network behavior combining identify and request-response.

use libp2p::{
    identify, request_response, gossipsub,
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
    /// Identify protocol for peer discovery
    pub identify: identify::Behaviour,
    /// Request-response for proof exchange
    pub request_response: request_response::cbor::Behaviour<ProofRequest, ProofResponse>,
    /// Gossipsub for threat intelligence
    pub gossipsub: gossipsub::Behaviour,
}

/// Events emitted by the SODS behavior.
#[derive(Debug)]
pub enum SodsBehaviourEvent {
    /// Identify event (peer info exchange)
    Identify(identify::Event),
    /// Request-response event
    RequestResponse(request_response::Event<ProofRequest, ProofResponse>),
    /// Gossipsub event
    Gossipsub(gossipsub::Event),
}

impl From<identify::Event> for SodsBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        SodsBehaviourEvent::Identify(event)
    }
}

impl From<request_response::Event<ProofRequest, ProofResponse>> for SodsBehaviourEvent {
    fn from(event: request_response::Event<ProofRequest, ProofResponse>) -> Self {
        SodsBehaviourEvent::RequestResponse(event)
    }
}

impl From<gossipsub::Event> for SodsBehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        SodsBehaviourEvent::Gossipsub(event)
    }
}

impl SodsBehaviour {
    /// Create a new SODS behavior with the given keypair.
    pub fn new(keypair: &libp2p::identity::Keypair) -> Self {
        // Identify config
        let identify = identify::Behaviour::new(
            identify::Config::new(
                "/sods/1.0.0".to_string(),
                keypair.public(),
            )
            .with_agent_version("sods/0.2.0".to_string()),
        );

        // Request-response config using CBOR codec
        let request_response = request_response::cbor::Behaviour::new(
            [(sods_protocol(), request_response::ProtocolSupport::Full)],
            request_response::Config::default(),
        );

        // Gossipsub config
        let message_authenticity = gossipsub::MessageAuthenticity::Signed(keypair.clone());
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("Valid config");

        let gossipsub = gossipsub::Behaviour::new(message_authenticity, gossipsub_config)
            .expect("Correct configuration");

        Self {
            identify,
            request_response,
            gossipsub,
        }
    }
}
