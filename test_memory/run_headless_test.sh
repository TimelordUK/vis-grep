#!/bin/bash

echo "=== Running Headless Memory Test ==="

# Create test directory
TEST_DIR="../test_logs"
mkdir -p "$TEST_DIR"

# Test files
LOG_FILE="$TEST_DIR/test.log"

# Clear log
> "$LOG_FILE"

# Start log generator in background
echo "Starting log generator..."
python3 generate_logs.py "$LOG_FILE" --rate 100 --burst &
LOG_GEN_PID=$!

# Give it a moment to start
sleep 2

# Run the memory test
echo -e "\nRunning memory test for 30 seconds...\n"
./target/release/headless_tail "$LOG_FILE" --buffer-size 10000 --duration 30

# Kill the log generator
echo -e "\nStopping log generator..."
kill $LOG_GEN_PID 2>/dev/null

# Show final log size
echo -e "\nFinal log file size:"
size=$(du -h "$LOG_FILE" | cut -f1)
lines=$(wc -l < "$LOG_FILE")
echo "  $LOG_FILE: $size ($lines lines)"

echo -e "\nTest complete!"