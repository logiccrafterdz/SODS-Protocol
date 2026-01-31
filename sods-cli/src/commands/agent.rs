use clap::{Args, Subcommand};

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
            if let Err(e) = crate::api::causal::start_server(serve_args.port).await {
                eprintln!("Error starting API server: {}", e);
                return 1;
            }
            0
        }
        #[cfg(not(feature = "api"))]
        AgentCommands::Serve(_) => {
            eprintln!("Error: API feature is disabled. Recompile with --features api");
            1
        }
    }
}
