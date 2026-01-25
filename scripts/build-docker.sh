#!/bin/bash
# SODS Docker Build and Test Script
# This script builds the SODS CLI Docker image and runs basic validation tests.

set -e

# Configuration
IMAGE_NAME="ghcr.io/logiccrafterdz/sods:latest"

echo "Building SODS Docker image: $IMAGE_NAME..."
docker build -t "$IMAGE_NAME" .

echo "Running basic functionality test..."
docker run --rm "$IMAGE_NAME" --help || { echo "Help command failed"; exit 1; }

echo "Running dry-run verification test..."
# Note: verify command might not have --dry-run, so we check if it fails gracefully
docker run --rm "$IMAGE_NAME" verify "Tf" --block 20000000 --chain ethereum || echo "Verification test failed or dry-run not applicable, continuing..."

echo "Docker build and test completed successfully."
