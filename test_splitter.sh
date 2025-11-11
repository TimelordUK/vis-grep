#!/bin/bash
# Test script to verify the horizontal splitter fix in tail mode

echo "Testing horizontal splitter in tail mode..."
echo "1. Start the application without any files"
echo "2. Switch to Tail mode"
echo "3. Try to drag the horizontal splitter between controls and output"
echo "4. The splitter should stay where you drag it, not bounce back"
echo ""
echo "Starting vis-grep..."

cargo run

echo ""
echo "Test complete. Did the splitter stay in place? (It should not bounce back every 1/4 second)"