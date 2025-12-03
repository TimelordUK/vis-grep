#!/bin/bash
# Interactive test script for idle monitoring

echo "Interactive Idle Monitor Test"
echo "============================"
echo ""
echo "This test uses a 2-minute idle timeout with 30-second grace period."
echo ""
echo "Test scenarios:"
echo "1. Grace period countdown (30 seconds)"
echo "2. After grace period: Idle countdown (2 minutes)"
echo "3. Any mouse click or keyboard input should reset the timer"
echo ""
echo "To enable debug logging and see activity detection:"
echo "  export RUST_LOG=debug"
echo ""

# Check if we should enable debug logging
if [ "$1" == "--debug" ]; then
    export RUST_LOG=debug
    echo "Debug logging enabled"
fi

# Make sure we have the test config
if [ ! -f test_idle_config.yaml ]; then
    echo "Error: test_idle_config.yaml not found"
    exit 1
fi

# Backup existing config if present
if [ -f ~/.config/vis-grep/config.yaml ]; then
    cp ~/.config/vis-grep/config.yaml ~/.config/vis-grep/config.yaml.bak
    echo "Backed up existing config"
fi

# Install test config
mkdir -p ~/.config/vis-grep
cp test_idle_config.yaml ~/.config/vis-grep/config.yaml

echo ""
echo "Starting vis-grep with idle monitoring..."
echo "Try clicking or typing to reset the timer!"
echo ""

# Run vis-grep
if [ -f ./target/release/vis-grep ]; then
    ./target/release/vis-grep -l test_tree_layout.yaml
else
    echo "Error: Release build not found. Run: cargo build --release"
    exit 1
fi

# Restore original config
if [ -f ~/.config/vis-grep/config.yaml.bak ]; then
    mv ~/.config/vis-grep/config.yaml.bak ~/.config/vis-grep/config.yaml
    echo "Restored original config"
fi