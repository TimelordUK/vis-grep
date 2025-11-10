#!/bin/bash
# Test panel layout issues with and without files

echo "Panel Layout Test - Checking splitter behavior"
echo "============================================="
echo ""

# Test 1: No files (should show splitter)
echo "Test 1: Starting vis-grep in tail mode with NO files"
echo "Expected: Should see working splitter between panels"
echo "Command: ./run.sh -f"
echo ""
read -p "Press ENTER to start test 1..."

./run.sh -f &
VIS_PID=$!

echo ""
read -p "Check if splitter works. Press ENTER when done..."
kill $VIS_PID 2>/dev/null || true

# Test 2: With files (reported splitter doesn't work)
echo ""
echo "Test 2: Starting vis-grep in tail mode WITH files"
echo "Expected: Splitter should still work"
echo ""

# Create a couple of test files if they don't exist
mkdir -p test_logs
echo "Creating test files..."
echo "Test log line 1" > test_logs/panel_test_1.log
echo "Test log line 2" > test_logs/panel_test_2.log

echo "Command: ./run.sh -f test_logs/panel_test_1.log test_logs/panel_test_2.log"
echo ""
read -p "Press ENTER to start test 2..."

./run.sh -f test_logs/panel_test_1.log test_logs/panel_test_2.log &
VIS_PID=$!

echo ""
echo "Check if:"
echo "1. Splitter is visible and draggable"
echo "2. Left panel can be resized smaller"
echo "3. Horizontal scrolling works on left panel"
echo "4. Preview panel doesn't go black when resizing"
echo ""
read -p "Press ENTER when done testing..."

kill $VIS_PID 2>/dev/null || true

# Cleanup
rm -f test_logs/panel_test_*.log

echo ""
echo "Test complete."