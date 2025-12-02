#!/bin/bash

echo "=== Comparing Memory Usage: Headless vs GUI ==="
echo
echo "This test demonstrates that the massive memory growth is from"
echo "egui rendering thousands of UI elements, not the core logic."
echo

# Setup
TEST_DIR="../test_logs"
mkdir -p "$TEST_DIR"
LOG_FILE="$TEST_DIR/comparison_test.log"

# Clear log
> "$LOG_FILE"

# Start log generation
echo "1. Starting log generation (100 lines/sec)..."
python3 generate_logs.py "$LOG_FILE" --rate 100 --burst &
LOG_PID=$!
sleep 3

# Run headless test
echo -e "\n2. Running HEADLESS test for 30 seconds..."
echo "   (Watch the memory usage)"
echo ""
timeout 30 ./target/release/headless_tail "$LOG_FILE" --buffer-size 10000 --duration 30 2>&1 | grep -E "Memory:|Test Complete" &
HEADLESS_PID=$!

# Wait for headless test
wait $HEADLESS_PID

# Now run the real vis-grep
echo -e "\n3. Now start the REAL vis-grep GUI with the same file:"
echo "   cd .."
echo "   cargo run --release -- --tail $LOG_FILE"
echo ""
echo "   Watch the memory usage in your system monitor!"
echo "   It will grow MUCH faster than the headless test."
echo ""
echo "4. Expected results:"
echo "   - Headless: ~10-20MB growth"
echo "   - GUI: 100s of MB growth"
echo "   - Difference: egui rendering overhead"

# Keep generating logs
echo -e "\nLog generation continues. Press Ctrl+C to stop..."

# Get log stats
while true; do
    sleep 5
    lines=$(wc -l < "$LOG_FILE" 2>/dev/null || echo 0)
    size_kb=$(du -k "$LOG_FILE" 2>/dev/null | cut -f1)
    echo -e "\rLog file: ${size_kb}KB (${lines} lines)   "
done

# Cleanup on exit
trap "kill $LOG_PID 2>/dev/null" EXIT