//! Chain configuration.

/// Supported blockchain configuration.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub name: &'static str,
    pub chain_id: u64,
    pub default_rpc: &'static str,
    pub default_ws: Option<&'static str>,
    pub description: &'static str,
}

/// All supported chains.
pub const CHAINS: &[ChainConfig] = &[
    ChainConfig {
        name: "sepolia",
        chain_id: 11155111,
        default_rpc: "https://ethereum-sepolia.publicnode.com",
        default_ws: Some("wss://ethereum-sepolia-rpc.publicnode.com"), // Heuristic guess or check user request? Prompt didn't specify Sepolia WS, but easy to check. Prompt listed Eth, Base, Arb. I'll stick to prompt for those.
        description: "Ethereum Sepolia Testnet",
    },
    ChainConfig {
        name: "ethereum",
        chain_id: 1,
        default_rpc: "https://eth.llamarpc.com",
        default_ws: Some("wss://ethereum-rpc.publicnode.com"),
        description: "Ethereum Mainnet",
    },
    ChainConfig {
        name: "base",
        chain_id: 8453,
        default_rpc: "https://base.publicnode.com",
        default_ws: Some("wss://base-rpc.publicnode.com"),
        description: "Base Mainnet (Coinbase L2)",
    },
    ChainConfig {
        name: "arbitrum",
        chain_id: 42161,
        default_rpc: "https://arbitrum.publicnode.com",
        default_ws: Some("wss://arbitrum-one-rpc.publicnode.com"),
        description: "Arbitrum One",
    },
    ChainConfig {
        name: "optimism",
        chain_id: 10,
        default_rpc: "https://optimism.publicnode.com",
        default_ws: None, // Not specified in prompt
        description: "Optimism Mainnet",
    },
    ChainConfig {
        name: "polygon-zkevm",
        chain_id: 1101,
        default_rpc: "https://zkevm-rpc.com",
        default_ws: None,
        description: "Polygon zkEVM",
    },
    ChainConfig {
        name: "scroll",
        chain_id: 534352,
        default_rpc: "https://rpc.scroll.io",
        default_ws: None,
        description: "Scroll zkEVM",
    },
];

/// Get chain config by name.
pub fn get_chain(name: &str) -> Option<&'static ChainConfig> {
    CHAINS.iter().find(|c| c.name.eq_ignore_ascii_case(name))
}

/// Supported behavioral symbols.
pub const SYMBOLS: &[(&str, &str)] = &[
    ("Tf", "ERC20 Transfer"),
    ("Dep", "WETH Deposit"),
    ("Wdw", "WETH Withdrawal"),
    ("Sw", "Uniswap V2 Swap"),
    ("LP+", "Uniswap V2 Mint (Add Liquidity)"),
    ("LP-", "Uniswap V2 Burn (Remove Liquidity)"),
];

/// Check if a symbol is supported.
pub fn is_symbol_supported(symbol: &str) -> bool {
    SYMBOLS.iter().any(|(s, _)| *s == symbol)
}
