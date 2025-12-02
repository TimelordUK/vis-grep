#!/bin/bash

echo "=== Aggressive Memory Test - Simulating Real-World High Volume ==="
echo

# Create test directory
TEST_DIR="../test_logs"
mkdir -p "$TEST_DIR"

# Multiple log files to simulate real scenario
LOG_FILES=(
    "$TEST_DIR/app1.log"
    "$TEST_DIR/app2.log"
    "$TEST_DIR/app3.log"
    "$TEST_DIR/app4.log"
    "$TEST_DIR/app5.log"
)

# Clear all logs
for file in "${LOG_FILES[@]}"; do
    > "$file"
done

echo "Test Configuration:"
echo "  - 5 log files"
echo "  - 300 lines/sec per file (1,500 lines/sec total)"
echo "  - Larger log entries (more 'huge' type)"
echo "  - Running for 60 seconds"
echo ""

# Start VERY aggressive log generation
for file in "${LOG_FILES[@]}"; do
    # Use higher rate and more huge logs
    python3 generate_logs.py "$file" --rate 300 --burst --template mixed &
done

# Store PIDs to kill later
jobs -p > /tmp/log_pids.txt

# Let logs accumulate
echo "Pre-generating some log data..."
sleep 3

echo "Starting memory test..."
echo "Watch for significant memory growth over 60 seconds:"
echo ""

# Run test for 60 seconds
./target/release/headless_tail "${LOG_FILES[@]}" --buffer-size 10000 --duration 60

# Kill all log generators
while read pid; do
    kill $pid 2>/dev/null
done < /tmp/log_pids.txt
rm -f /tmp/log_pids.txt

# Show final statistics
echo -e "\n=== Final Statistics ==="
total_lines=0
total_size_kb=0

for file in "${LOG_FILES[@]}"; do
    if [[ -f "$file" ]]; then
        size_kb=$(du -k "$file" | cut -f1)
        lines=$(wc -l < "$file" 2>/dev/null || echo 0)
        echo "  $(basename "$file"): ${size_kb}KB (${lines} lines)"
        total_lines=$((total_lines + lines))
        total_size_kb=$((total_size_kb + size_kb))
    fi
done

echo -e "\nTotal data processed:"
echo "  - Files: ${#LOG_FILES[@]}"
echo "  - Lines: $total_lines"
echo "  - Size: ${total_size_kb}KB ($((total_size_kb / 1024))MB)"
if [[ $total_lines -gt 0 ]]; then
    echo "  - Avg line size: $((total_size_kb * 1024 / total_lines)) bytes"
    echo "  - Lines/sec: $((total_lines / 60))"
fi