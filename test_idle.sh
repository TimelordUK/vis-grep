#!/bin/bash
# Test script for idle monitoring feature

echo "Testing vis-grep idle monitoring..."
echo "Creating test config with 2-minute idle timeout..."

# Backup existing config if present
if [ -f ~/.config/vis-grep/config.yaml ]; then
    cp ~/.config/vis-grep/config.yaml ~/.config/vis-grep/config.yaml.bak
    echo "Backed up existing config to config.yaml.bak"
fi

# Copy test config
mkdir -p ~/.config/vis-grep
cp test_idle_config.yaml ~/.config/vis-grep/config.yaml
echo "Test config installed with 2-minute idle timeout"

echo ""
echo "Starting vis-grep in tail mode..."
echo "- The status bar should show: 'Auto-shutdown in: 1m 59s'"
echo "- Timer will count down from 2 minutes"
echo "- Any mouse click or keyboard input will reset the timer"
echo "- App will auto-close when timer reaches zero"
echo ""
echo "Press Ctrl+C to cancel test"
echo ""

# Run vis-grep in tail mode with the test layout
./target/release/vis-grep -l test_tree_layout.yaml

# Restore original config if backed up
if [ -f ~/.config/vis-grep/config.yaml.bak ]; then
    mv ~/.config/vis-grep/config.yaml.bak ~/.config/vis-grep/config.yaml
    echo "Restored original config"
fi