#!/bin/bash

# Script to test memory growth in tail mode by generating lots of data

echo "=== Testing vis-grep memory growth ==="
echo

# Create test directory
TEST_DIR="test_memory"
mkdir -p "$TEST_DIR"

# Generate initial large files
echo "Creating test files..."
for i in {1..5}; do
    FILE="$TEST_DIR/test$i.log"
    echo "Creating $FILE..."
    
    # Generate 10MB of initial data per file
    for j in {1..100000}; do
        echo "[$(date --iso-8601=seconds)] INFO [TestModule$i] Initial line $j with some padding to make it longer: $(head -c 50 /dev/urandom | base64)" >> "$FILE"
    done
done

# Create layout file
cat > "$TEST_DIR/layout.yaml" <<EOF
version: 1
groups:
  - name: "Memory Test"
    files:
EOF

for i in {1..5}; do
    echo "      - path: \"$(pwd)/$TEST_DIR/test$i.log\"" >> "$TEST_DIR/layout.yaml"
done

echo "Files created. Starting aggressive appender..."

# Function to aggressively append data
aggressive_append() {
    while true; do
        for i in {1..5}; do
            FILE="$TEST_DIR/test$i.log"
            # Append 100 lines per iteration
            for j in {1..100}; do
                echo "[$(date --iso-8601=seconds)] WARN [MemoryTest] Heavy load test line with random data: $(head -c 100 /dev/urandom | base64)" >> "$FILE"
            done
        done
        # Very short sleep to stress test
        sleep 0.1
    done
}

# Start aggressive appender in background
aggressive_append &
APPEND_PID=$!

echo
echo "Background appender started (PID: $APPEND_PID)"
echo
echo "To monitor memory growth:"
echo "  RUST_LOG=info ./target/release/vis-grep --tail-layout $TEST_DIR/layout.yaml"
echo
echo "Look for:"
echo "  - Memory usage in status bar (bottom right)"
echo "  - Memory log messages showing growth"
echo "  - Performance degradation as memory grows"
echo
echo "Press Ctrl+C to stop"

cleanup() {
    echo
    echo "Stopping appender..."
    kill $APPEND_PID 2>/dev/null
    echo "To clean up: rm -rf $TEST_DIR"
}

trap cleanup EXIT

# Keep script running
while true; do
    sleep 1
done