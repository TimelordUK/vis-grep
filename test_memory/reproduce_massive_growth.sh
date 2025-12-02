#!/bin/bash

echo "=== Reproducing Massive Memory Growth ==="
echo "This test simulates the scenario where memory jumps from 400MB to 1GB"
echo

# Setup
TEST_DIR="../test_logs"
mkdir -p "$TEST_DIR"
LOG_FILE="$TEST_DIR/massive_growth.log"
> "$LOG_FILE"

echo "Configuration:"
echo "  - Heavy logs with unique content (defeats string deduplication)"
echo "  - High rate: 500 lines/sec"
echo "  - Large payloads with UUIDs"
echo ""

# Start heavy log generation
echo "Starting heavy log generation..."
python3 generate_heavy_logs.py "$LOG_FILE" --rate 500 --burst --clear &
LOG_PID=$!

# Let it accumulate
sleep 3

echo "Running memory test for 60 seconds..."
echo "WATCH THE MEMORY COLUMN - it should grow significantly!"
echo ""
echo "Time     Lines   Buffer  Memory   Delta    Peak    Cache"
echo "-------- ------- ------- -------- -------- ------- ------"

# Run with detailed output
./target/release/headless_tail "$LOG_FILE" --buffer-size 10000 --duration 60

# Cleanup
kill $LOG_PID 2>/dev/null

# Analysis
echo -e "\n=== Analysis ==="
size_mb=$(du -m "$LOG_FILE" | cut -f1)
lines=$(wc -l < "$LOG_FILE")
echo "Generated: ${size_mb}MB in $lines lines"

if [[ $lines -gt 0 ]]; then
    avg_size=$((size_mb * 1024 * 1024 / lines))
    echo "Average line size: $avg_size bytes"
fi

echo -e "\n=== Why This Causes Massive Growth ==="
echo "1. Each line contains multiple UUIDs (36 chars each)"
echo "2. JSON payloads with unique IDs defeat string caching"
echo "3. Large stack traces and SQL queries"
echo "4. String cache fills with unique strings that are never reused"
echo "5. No eviction means cache grows until it hits 10K limit"
echo ""
echo "With 10K cache entries of ~500-1000 bytes each = 5-10MB"
echo "Plus buffer of 10K lines at ~1KB each = 10MB"
echo "Plus overhead and fragmentation = easily 50-100MB+"
echo ""
echo "At high rates, this compounds quickly!"