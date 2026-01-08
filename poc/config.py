"""
SODS PoC v0.1 — Configuration and Constants

Hardcoded parameters for the proof of concept.
Chain: Ethereum Sepolia (testnet)
Block: #5,000,000
"""

import os
from pathlib import Path
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

# =============================================================================
# INFURA CONFIGURATION
# =============================================================================

INFURA_PROJECT_ID = os.getenv("INFURA_PROJECT_ID", "")
INFURA_RPC_URL = f"https://sepolia.infura.io/v3/{INFURA_PROJECT_ID}"

# =============================================================================
# BLOCK CONFIGURATION
# =============================================================================

TARGET_BLOCK = 10_002_322  # Block with Deposit events
CHAIN_NAME = "sepolia"

# =============================================================================
# EVENT TOPIC HASHES (Keccak-256 of event signature)
# =============================================================================

# ERC20 Transfer(address indexed from, address indexed to, uint256 value)
TRANSFER_TOPIC = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"

# Uniswap V2 Pair Mint(address indexed sender, uint256 amount0, uint256 amount1)
MINT_TOPIC = "0x4c209b5fc8ad50758f13e2e1088ba56a560dff690a1c6fef26391d14d59cf6ad"

# WETH Deposit(address indexed dst, uint256 wad) — Very active on Sepolia!
DEPOSIT_TOPIC = "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c"

# WETH Withdrawal(address indexed src, uint256 wad)
WITHDRAWAL_TOPIC = "0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65"

# =============================================================================
# SYMBOL REGISTRY
# Maps topic hashes to canonical symbols
# =============================================================================

SYMBOL_REGISTRY = {
    TRANSFER_TOPIC: "Tf",       # ERC20 Transfer
    MINT_TOPIC: "LP+",          # Uniswap V2 Add Liquidity (Mint)
    DEPOSIT_TOPIC: "Dep",       # WETH Deposit (wrap ETH)
    WITHDRAWAL_TOPIC: "Wdw",    # WETH Withdrawal (unwrap ETH)
}

# Reverse lookup: symbol → topic
TOPIC_REGISTRY = {v: k for k, v in SYMBOL_REGISTRY.items()}

# =============================================================================
# UNISWAP V2 CONFIGURATION (Sepolia)
# =============================================================================

UNISWAP_V2_FACTORY = "0xF62c03E08ada871A0bEb309762E260a7a6a880E6"

# =============================================================================
# OUTPUT PATHS
# =============================================================================

PROJECT_ROOT = Path(__file__).parent  # poc/ directory
BMR_FILE = PROJECT_ROOT / "bmr.bin"
PROOFS_DIR = PROJECT_ROOT / "proofs"

# Ensure proofs directory exists
PROOFS_DIR.mkdir(exist_ok=True)

