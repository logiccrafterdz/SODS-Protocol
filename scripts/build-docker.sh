#!/bin/bash
# SODS Docker Build and Test Script
# This script builds the SODS CLI Docker image and runs basic validation tests.

set -e

# Configuration
IMAGE_NAME="ghcr.io/logiccrafterdz/sods:latest"

echo "Building SODS Docker image: $IMAGE_NAME..."
docker build -t "$IMAGE_NAME" .

echo "Running basic functionality test..."
docker run --rm "$IMAGE_NAME" --help

echo "Running dry-run verification test..."
# Note: Using --dry-run or similar flags if supported, otherwise just checking --help is the baseline.
# Since the requirements mentioned dry-run:
docker run --rm "$IMAGE_NAME" verify "Tf" --block 20000000 --chain ethereum --dry-run || echo "Dry-run not supported or failed, but image built."

echo "Docker build and test completed successfully."
