use clap::Args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct RegisterAgentArgs {
    /// Name of the agent
    #[arg(long)]
    pub name: String,

    /// Description of the agent's purpose
    #[arg(long)]
    pub description: String,

    /// Endpoint for the REST API service
    #[arg(long)]
    pub endpoint: String,

    /// Output directory for the registration.json file
    #[arg(long, default_value = ".")]
    pub output_dir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentService {
    pub name: String,
    pub endpoint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentExtensions {
    #[serde(rename = "behavioralProof")]
    pub behavioral_proof: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRegistration {
    #[serde(rename = "type")]
    pub registration_type: String,
    pub name: String,
    pub description: String,
    pub services: Vec<AgentService>,
    #[serde(rename = "supportedTrust")]
    pub supported_trust: Vec<String>,
    pub extensions: AgentExtensions,
}

pub async fn run(args: RegisterAgentArgs) -> i32 {
    let registration = AgentRegistration {
        registration_type: "https://eips.ethereum.org/EIPS/eip-8004#registration-v1".to_string(),
        name: args.name,
        description: args.description,
        services: vec![AgentService {
            name: "REST API".to_string(),
            endpoint: args.endpoint.clone(),
        }],
        supported_trust: vec!["reputation".to_string(), "zk-proofs".to_string()],
        extensions: AgentExtensions {
            behavioral_proof: format!("{}/causal/proof/{{agentId}}", args.endpoint),
        },
    };

    let json = match serde_json::to_string_pretty(&registration) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Error serializing registration: {}", e);
            return 1;
        }
    };

    let file_path = args.output_dir.join("registration.json");
    if let Err(e) = fs::write(&file_path, json) {
        eprintln!("Error writing registration file: {}", e);
        return 1;
    }

    println!("✅ Agent registration file created at: {}", file_path.display());
    println!("🚀 You can now host this file on IPFS or your service endpoint.");
    
    0
}
