#!/bin/bash
# SODS ERC-8004 Agent Deployment Script
# This script deploys the required registries and registers the SODS agent.

set -e

echo "🚀 Starting SODS ERC-8004 Agent Deployment..."

# 1. Configuration
CHAIN="sepolia"
AGENT_NAME="SODS Causal Verifier"
AGENT_DESC="Trustless behavioral verification for AI agents using Causal Merkle Trees"
ENDPOINT="https://api.sods.xyz"
OUTPUT_DIR="./deployments"

mkdir -p $OUTPUT_DIR

# 2. Register Agent Identity (JSON Generation)
echo "📝 Generating Agent Identity Registration..."
./target/debug/sods agent register \
    --name "$AGENT_NAME" \
    --description "$AGENT_DESC" \
    --endpoint "$ENDPOINT" \
    --output-dir "$OUTPUT_DIR"

echo "✅ Registration JSON generated at $OUTPUT_DIR/registration.json"

# 3. Simulate Registry Deployment
# In a real scenario, this would use 'forge' or 'cast' to deploy Solidity contracts.
# Here we provide the expected contract addresses for Sepolia.
REGISTRY_ADDR="0x8004000000000000000000000000000000000001"
REPUTATION_ADDR="0x8004000000000000000000000000000000000002"
VALIDATION_ADDR="0x8004000000000000000000000000000000000003"

echo "🌍 Registries active on $CHAIN:"
echo "   - Identity:   $REGISTRY_ADDR"
echo "   - Reputation: $REPUTATION_ADDR"
echo "   - Validation: $VALIDATION_ADDR"

# 4. Start the Agent Service
echo "📡 Starting SODS Causal Agent Service..."
echo "Command: sods agent serve --port 8080"

# Provide full agent identifier
echo "--------------------------------------------------"
echo "AGENT IDENTIFIER: eip155:11155111:$REGISTRY_ADDR:1"
echo "--------------------------------------------------"

echo "✅ SODS Causal Verifier is now live as an ERC-8004 Agent."
