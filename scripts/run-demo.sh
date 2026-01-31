#!/bin/bash
# SODS MEV Protection Demo Automation Script

set -e

echo "--------------------------------------------------"
echo "SODS REAL-WORLD USE CASE: MEV PROTECTION"
echo "--------------------------------------------------"

# 1. Build components
echo "🛠️ Building SODS CLI and DeFi Simulator..."
cargo build -p sods-cli --features api > /dev/null 2>&1
cargo build -p defi-protocol > /dev/null 2>&1

# 2. Start SODS Agent Service in the background
echo "📡 Starting SODS Agent Service (Port 8080)..."
./target/debug/sods agent serve --port 8080 &
AGENT_PID=$!

# Wait for server to start
sleep 3

# 3. Run the DeFi Protocol Simulator
echo "🤖 Running AI Agent Trading Simulation..."
./target/debug/defi-protocol

# 4. Cleanup
echo "🧹 Cleaning up..."
kill $AGENT_PID

echo "--------------------------------------------------"
echo "✅ Demo execution complete."
echo "--------------------------------------------------"
