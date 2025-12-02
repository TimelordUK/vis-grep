#!/bin/bash

echo "=== Demonstrating Memory Growth Issue ==="
echo 

# Create test log
LOG_FILE="../test_logs/demo.log"
mkdir -p ../test_logs
> "$LOG_FILE"

# Start aggressive log generation
echo "Starting aggressive log generation:"
echo "  - Mixed log sizes (60% short, 25% medium, 10% long, 5% huge)"
echo "  - 100 lines/sec with bursts"
echo ""

# Generate logs in background
python3 generate_logs.py "$LOG_FILE" --rate 100 --burst --template mixed &
LOG_PID=$!

# Let it generate some initial data
sleep 2

echo "Running memory test for 30 seconds..."
echo "Watch how memory grows from initial ~2MB:"
echo ""

# Run the headless test
./target/release/headless_tail "$LOG_FILE" --buffer-size 10000 --duration 30

# Stop log generation
kill $LOG_PID 2>/dev/null

# Show stats
echo -e "\nTest complete!"
size_kb=$(du -k "$LOG_FILE" | cut -f1)
lines=$(wc -l < "$LOG_FILE")
echo "Log file: ${size_kb}KB ($lines lines)"
echo "Average line: $((size_kb * 1024 / lines)) bytes"

# Key observations
echo -e "\n=== Key Observations ==="
echo "1. Memory grows continuously even with a fixed 10K line buffer"
echo "2. String cache can grow to 10K entries of arbitrary size"
echo "3. No line length limits means huge lines consume lots of memory"
echo "4. Buffer capacity may not shrink aggressively enough"

echo -e "\n=== Suggested Fixes ==="
echo "1. Implement line truncation (e.g., max 1KB per line)"
echo "2. Add LRU eviction to string cache"
echo "3. Periodic cache cleanup based on memory pressure"
echo "4. More aggressive buffer capacity management"