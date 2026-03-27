# ============================================================
# SODS Protocol - Zero-Knowledge Build Script (Windows)
# ============================================================
# This script compiles the sods-zk guest methods for the
# RISC Zero zkVM on Windows systems.
#
# Prerequisites:
#   1. Install RISC Zero: cargo install cargo-risczero
#   2. Install the toolchain: cargo risczero install
#
# Usage:
#   .\build-zk.ps1
# ============================================================

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ZkDir = Join-Path $ScriptDir "sods-zk"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  SODS Protocol - ZK Build" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host ""

# 1. Check prerequisites
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: 'cargo' not found. Please install Rust first." -ForegroundColor Red
    exit 1
}

try { cargo risczero --version 2>&1 | Out-Null } catch {
    Write-Host "  'cargo-risczero' not found. Installing..." -ForegroundColor Yellow
    cargo install cargo-risczero
    cargo risczero install
    Write-Host "  RISC Zero toolchain installed." -ForegroundColor Green
}

# 2. Build guest methods
Write-Host ""
Write-Host "  Building sods-zk guest methods..." -ForegroundColor Yellow
Write-Host "   Directory: $ZkDir"
Write-Host ""

Push-Location $ZkDir
try {
    cargo build --release
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "  ZK guest methods built successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "  Artifacts:" -ForegroundColor Cyan
Write-Host "   $ZkDir\target\release\"
Write-Host ""
Write-Host "  You can now use 'sods zk-prove' with the --zk feature enabled." -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
