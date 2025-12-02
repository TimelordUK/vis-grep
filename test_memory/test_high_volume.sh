#!/bin/bash

echo "=== High Volume Memory Test ==="

# Create test directory
TEST_DIR="../test_logs"
mkdir -p "$TEST_DIR"

# Test files
LOG_FILES=(
    "$TEST_DIR/app1.log"
    "$TEST_DIR/app2.log"
    "$TEST_DIR/app3.log"
)

# Clear logs
for file in "${LOG_FILES[@]}"; do
    > "$file"
done

# Start high-volume log generation
echo "Starting high-volume log generation..."
echo "  - 3 files"
echo "  - 200 lines/sec per file (600 total)"
echo "  - Burst mode enabled"
echo ""

python3 generate_logs.py "${LOG_FILES[@]}" --rate 200 --burst --template mixed &
LOG_GEN_PID=$!

# Give it time to generate initial data
sleep 3

# Run the memory test for longer
echo "Running memory test for 60 seconds..."
echo "Watch the memory growth pattern below:"
echo ""

./target/release/headless_tail "${LOG_FILES[@]}" --buffer-size 10000 --duration 60

# Kill the log generator
kill $LOG_GEN_PID 2>/dev/null

# Show final stats
echo -e "\nFinal log file stats:"
total_lines=0
total_size=0
for file in "${LOG_FILES[@]}"; do
    size_kb=$(du -k "$file" | cut -f1)
    lines=$(wc -l < "$file")
    echo "  $(basename "$file"): ${size_kb}KB ($lines lines)"
    total_lines=$((total_lines + lines))
    total_size=$((total_size + size_kb))
done

echo -e "\nTotal: ${total_size}KB ($total_lines lines)"
echo "Average line size: $((total_size * 1024 / total_lines)) bytes"