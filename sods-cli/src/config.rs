//! Chain configuration.

/// Supported blockchain configuration.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub name: &'static str,
    pub chain_id: u64,
    pub rpc_urls: &'static [&'static str],
    pub default_ws: Option<&'static str>,
    pub description: &'static str,
}

/// All supported chains.
pub const CHAINS: &[ChainConfig] = &[
    ChainConfig {
        name: "sepolia",
        chain_id: 11155111,
        rpc_urls: &[
            "https://ethereum-sepolia.publicnode.com",
            "https://rpc.sepolia.org",
            "https://1rpc.io/sepolia"
        ],
        default_ws: Some("wss://ethereum-sepolia-rpc.publicnode.com"),
        description: "Ethereum Sepolia Testnet",
    },
    ChainConfig {
        name: "ethereum",
        chain_id: 1,
        rpc_urls: &[
            "https://eth.llamarpc.com",
            "https://ethereum-rpc.publicnode.com",
            "https://1rpc.io/eth"
        ],
        default_ws: Some("wss://ethereum-rpc.publicnode.com"),
        description: "Ethereum Mainnet",
    },
    ChainConfig {
        name: "base",
        chain_id: 8453,
        rpc_urls: &[
            "https://base.publicnode.com",
            "https://mainnet.base.org",
            "https://1rpc.io/base"
        ],
        default_ws: Some("wss://base-rpc.publicnode.com"),
        description: "Base Mainnet (Coinbase L2)",
    },
    ChainConfig {
        name: "arbitrum",
        chain_id: 42161,
        rpc_urls: &[
            "https://arbitrum.publicnode.com",
            "https://arb1.arbitrum.io/rpc",
            "https://1rpc.io/arb"
        ],
        default_ws: Some("wss://arbitrum-one-rpc.publicnode.com"),
        description: "Arbitrum One",
    },
    ChainConfig {
        name: "optimism",
        chain_id: 10,
        rpc_urls: &[
            "https://optimism.publicnode.com",
            "https://mainnet.optimism.io",
            "https://1rpc.io/op"
        ],
        default_ws: None,
        description: "Optimism Mainnet",
    },
    ChainConfig {
        name: "polygon-zkevm",
        chain_id: 1101,
        rpc_urls: &[
            "https://zkevm-rpc.com",
            "https://polygon-zkevm.publicnode.com",
            "https://1rpc.io/polygon/zkevm"
        ],
        default_ws: None,
        description: "Polygon zkEVM",
    },
    ChainConfig {
        name: "scroll",
        chain_id: 534352,
        rpc_urls: &[
            "https://rpc.scroll.io",
            "https://scroll-rpc.publicnode.com",
            "https://1rpc.io/scroll"
        ],
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
    ("Sw", "Uniswap V2/V3 Swap"),
    ("LP+", "Uniswap V2 Mint (Add Liquidity)"),
    ("LP-", "Uniswap V2 Burn (Remove Liquidity)"),
    ("MintNFT", "ERC721/ERC1155 Mint (Transfer from 0x0)"),
    ("BuyNFT", "NFT Purchase (Seaport OrderFulfilled)"),
    ("ListNFT", "NFT Listing (Blur OrdersMatched)"),
    ("BridgeIn", "L1→L2 Bridge Deposit (Optimism DepositFinalized)"),
    ("BridgeOut", "L2→L1 Bridge Withdrawal (Arbitrum/Scroll)"),
    ("Frontrun", "MEV Frontrun Pattern (Tf → Sw)"),
    ("Backrun", "MEV Backrun Pattern (Sw → Tf)"),
    ("Sandwich", "MEV Sandwich Pattern (Tf → Sw → Tf)"),
];

/// Check if a symbol is supported.
pub fn is_symbol_supported(symbol: &str) -> bool {
    SYMBOLS.iter().any(|(s, _)| *s == symbol)
}
