#!/bin/bash

# Memory test runner for vis-grep tail mode

echo "=== vis-grep Memory Test Runner ==="

# Create test log directory
TEST_DIR="test_logs"
mkdir -p "$TEST_DIR"

# Test files
LOG_FILES=(
    "$TEST_DIR/app1.log"
    "$TEST_DIR/app2.log" 
    "$TEST_DIR/app3.log"
)

# Clear existing logs
echo "Clearing existing log files..."
for file in "${LOG_FILES[@]}"; do
    > "$file"
done

# Build the headless test binary
echo "Building headless tail test..."
cd test_memory
cargo build --release
cd ..

# Start log generator in background
echo "Starting log generator..."
python3 test_memory/generate_logs.py "${LOG_FILES[@]}" --rate 100 --burst --clear &
LOG_GEN_PID=$!

# Give it a moment to start
sleep 2

# Run the memory test
echo -e "\nRunning memory test for 60 seconds...\n"
./target/release/headless_tail "${LOG_FILES[@]}" --buffer-size 10000 --duration 60

# Kill the log generator
echo -e "\nStopping log generator..."
kill $LOG_GEN_PID 2>/dev/null

# Show final log sizes
echo -e "\nFinal log file sizes:"
for file in "${LOG_FILES[@]}"; do
    size=$(du -h "$file" | cut -f1)
    lines=$(wc -l < "$file")
    echo "  $file: $size ($lines lines)"
done

echo -e "\nTest complete!"