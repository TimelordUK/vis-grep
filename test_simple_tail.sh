#!/bin/bash

# Simple test script for testing tail mode without tree layout
# Usage: ./test_simple_tail.sh [num_files]

NUM_FILES=${1:-3}
LOG_DIR="test_logs"

# Create log directory
mkdir -p "$LOG_DIR"

# Create test files
FILES=""
for (( i=1; i<=NUM_FILES; i++ )); do
    FILE="$LOG_DIR/simple_test_${i}.log"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting simple test log $i" > "$FILE"
    FILES="$FILES $FILE"
done

echo "Created $NUM_FILES test files"

# Append logs in background
(
    while true; do
        for (( i=1; i<=NUM_FILES; i++ )); do
            FILE="$LOG_DIR/simple_test_${i}.log"
            echo "[$(date '+%Y-%m-%d %H:%M:%S.%3N')] Test message $RANDOM" >> "$FILE"
        done
        sleep 0.5
    done
) &

APPENDER_PID=$!

cleanup() {
    kill $APPENDER_PID 2>/dev/null
    exit 0
}

trap cleanup EXIT INT TERM

# Run vis-grep in tail mode
echo "Starting vis-grep in tail mode with $NUM_FILES files"
echo "Press Ctrl+C to stop"

cargo build --release 2>/dev/null || cargo build

if [ -f target/release/vis-grep ]; then
    ./target/release/vis-grep -f $FILES
else
    ./target/debug/vis-grep -f $FILES
fi