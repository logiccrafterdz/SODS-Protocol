use std::sync::Arc;
use clap::{Args, Subcommand};
#[cfg(feature = "metrics")]
use crate::monitoring::metrics::AgentMetrics;

#[derive(Args, Debug)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommands,
}

#[derive(Subcommand, Debug)]
pub enum AgentCommands {
    /// Register the SODS agent as an ERC-8004 compliant entity
    Register(crate::commands::register_agent::RegisterAgentArgs),
    
    /// Start the ERC-8004 compliant REST API server
    #[cfg(feature = "api")]
    Serve(ServeArgs),
}

#[cfg(feature = "api")]
#[derive(Args, Debug)]
pub struct ServeArgs {
    /// Port to listen on
    #[arg(short, long, default_value_t = 8080)]
    pub port: u16,
}

pub async fn run(args: AgentArgs) -> i32 {
    match args.command {
        AgentCommands::Register(reg_args) => crate::commands::register_agent::run(reg_args).await,
        #[cfg(feature = "api")]
        AgentCommands::Serve(serve_args) => {
            let metrics = {
                 #[cfg(feature = "metrics")]
                 { AgentMetrics::new().ok().map(Arc::new) }
                 #[cfg(not(feature = "metrics"))]
                 { None }
            };

            if let Some(ref m) = metrics {
                let m_clone = m.clone();
                // Metrics are usually on a separate port or same. Let's use separate if port is provided, 
                // but here ServeArgs only has one port. The user probably wants metrics on the same or a default one.
                // For now, let's just use 9090 as default for metrics if not on same.
                tokio::spawn(m_clone.start_http_server(9090));
            }

            if let Err(e) = crate::api::causal::start_server(serve_args.port, metrics).await {
                eprintln!("Error starting API server: {}", e);
                return 1;
            }
            0
        }
    }
}
