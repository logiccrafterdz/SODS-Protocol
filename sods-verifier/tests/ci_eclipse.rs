use sods_p2p::network::MultiPathNetwork;
use libp2p::PeerId;
use std::collections::HashSet;

#[tokio::test]
async fn test_multi_path_confirmation() {
    // Create a mock MultiPathNetwork
    // Note: In real test, we would need a Swarm, but we can verify confirmation logic
    let keypair = libp2p::identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());
    let mut network = MultiPathNetwork {
        swarm: unsafe { std::mem::zeroed() }, // Mock placeholder
        transport_confirmations: std::collections::HashMap::new(),
    };

    // 1. Initially unconfirmed
    assert!(!network.is_peer_fully_confirmed(&peer_id));

    // 2. Add one transport confirmation
    let mut set1 = HashSet::new();
    set1.insert("tcp".to_string());
    network.transport_confirmations.insert(peer_id, set1);
    assert!(!network.is_peer_fully_confirmed(&peer_id));

    // 3. Add second independent transport
    network.transport_confirmations.get_mut(&peer_id).unwrap().insert("webrtc".to_string());
    assert!(network.is_peer_fully_confirmed(&peer_id));
    
    println!("✅ Multi-Path Confirmation Logic Verified.");
}
