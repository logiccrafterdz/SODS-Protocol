use ethers::types::Address;
use sods_causal::{
    CausalEvent, CausalEventRecorder, AgentBehaviorPattern,
    generate_behavioral_proof, CausalBehavioralProof
};
use serde::{Deserialize, Serialize};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting DeFi Protocol Simulator - AI Agent Behavioral Verification Demo");

    let agent_id = Address::random();
    println!("🤖 AI Trading Agent ID: {:?}", agent_id);

    // 1. Simulate Trading History (10 profitable trades)
    let mut recorder = CausalEventRecorder::new();
    println!("📝 Recording 10 profitable trades...");

    for i in 0..10 {
        let event = CausalEvent::builder()
            .agent_id(agent_id)
            .nonce(0)
            .sequence_index(i as u32)
            .event_type("trade_executed")
            .result("profit")
            .timestamp(1706720000 + (i as u64 * 60))
            .build()?;
        
        recorder.record_event(event)?;
    }

    // 2. Build Merkle Tree
    let tree = recorder.build_merkle_tree(&agent_id)?;
    println!("🌳 Causal Merkle Tree built. Root: {:?}", tree.root);

    // 3. Define Behavioral Pattern (MEV Protection)
    let pattern = AgentBehaviorPattern {
        event_type: "trade_executed".to_string(),
        result_filter: "profit".to_string(),
        min_count: 10,
        max_count: None,
        time_window: None,
    };

    // 4. Generate Behavioral Proof
    let now = 1706730000;
    let proof = generate_behavioral_proof(&tree, &pattern, now)?;
    println!("📄 Behavioral Proof generated for pattern: {:?}", pattern.event_type);

    // 5. Submit to SODS Agent API (Mocked call to our /causal/verify endpoint)
    println!("📡 Submitting proof to SODS Agent at http://localhost:8080/causal/verify...");
    
    let client = reqwest::Client::new();
    let res = client.post("http://localhost:8080/causal/verify")
        .json(&proof)
        .send()
        .await;

    match res {
        Ok(response) => {
            let body: serde_json::Value = response.json().await?;
            println!("✅ SODS Validation Response: {}", body);
            
            if body["valid"].as_bool() == Some(true) {
                println!("💰 ESCROW TRIGGER: Verification successful. Payment can be released!");
            } else {
                println!("❌ ESCROW TRIGGER: Verification failed. Payment blocked.");
            }
        }
        Err(_) => {
            println!("⚠️ SODS Agent not reachable. (Make sure 'sods agent serve' is running on port 8080)");
            println!("   Falling back to local verification for demonstration...");
            let is_valid = proof.verify(now);
            println!("✅ Local Verification result: {}", is_valid);
        }
    }

    // 6. Demonstrate Malicious Behavior (Tampered Proof)
    println!("\n🕵️ Simulating Malicious Behavior (Tampered History)...");
    let mut tampered_events = proof.matched_events.clone();
    tampered_events[0].result = "loss".to_string(); // Change a profit to a loss
    
    let tampered_proof = CausalBehavioralProof {
        pattern: proof.pattern.clone(),
        matched_events: tampered_events,
        event_proofs: proof.event_proofs.clone(),
        agent_root: proof.agent_root,
    };

    println!("❌ Verification result for tampered history: {}", tampered_proof.verify(now));

    println!("\n🏁 Demo completed.");
    Ok(())
}
