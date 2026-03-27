#!/usr/bin/env bash
# ============================================================
# SODS Protocol — Zero-Knowledge Build Script
# ============================================================
# This script compiles the sods-zk guest methods for the
# RISC Zero zkVM. It is intentionally separated from the
# main workspace build because the zkVM toolchain is heavy
# and requires the risc0 CLI to be installed.
#
# Prerequisites:
#   1. Install RISC Zero: cargo install cargo-risczero
#   2. Install the toolchain: cargo risczero install
#
# Usage:
#   chmod +x build-zk.sh && ./build-zk.sh
# ============================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZK_DIR="${SCRIPT_DIR}/sods-zk"

echo "============================================"
echo "  SODS Protocol — ZK Build"
echo "============================================"
echo ""

# 1. Check prerequisites
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: 'cargo' not found. Please install Rust first."
    exit 1
fi

if ! cargo risczero --version &> /dev/null 2>&1; then
    echo "⚠️  'cargo-risczero' not found. Installing..."
    cargo install cargo-risczero
    cargo risczero install
    echo "✅ RISC Zero toolchain installed."
fi

# 2. Build guest methods
echo ""
echo "🔧 Building sods-zk guest methods..."
echo "   Directory: ${ZK_DIR}"
echo ""

cd "${ZK_DIR}"
cargo build --release

echo ""
echo "✅ ZK guest methods built successfully!"
echo ""
echo "📦 Artifacts:"
echo "   ${ZK_DIR}/target/release/"
echo ""
echo "🚀 You can now use 'sods zk-prove' with the --zk feature enabled."
echo "============================================"
