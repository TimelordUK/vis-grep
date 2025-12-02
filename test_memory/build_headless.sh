#!/bin/bash

echo "Building standalone headless tail test..."

# Copy the standalone Cargo.toml
cp Cargo_standalone.toml Cargo.toml

# Build the standalone version
cargo build --release

echo "Build complete! Binary at: ../target/release/headless_tail"