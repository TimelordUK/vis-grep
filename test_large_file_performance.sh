#!/bin/bash

# Test script for vis-grep tail mode performance with large files

echo "=== Testing vis-grep tail mode performance with large files ==="
echo

# Create test directory
TEST_DIR="test_large_files"
mkdir -p "$TEST_DIR"

# Function to generate a large log file
generate_large_file() {
    local filename=$1
    local size_mb=$2
    local filepath="$TEST_DIR/$filename"
    
    echo "Generating $filename (${size_mb}MB)..."
    
    # Generate file with realistic log lines
    local lines_needed=$((size_mb * 1024 * 1024 / 100))  # ~100 bytes per line
    
    # Create initial content
    for i in $(seq 1 $lines_needed); do
        echo "[$(date --iso-8601=seconds)] INFO  [TestModule] Processing request ID=$i, status=SUCCESS, duration=${RANDOM}ms"
    done > "$filepath"
    
    local actual_size=$(stat -c%s "$filepath" 2>/dev/null || stat -f%z "$filepath" 2>/dev/null)
    local actual_mb=$((actual_size / 1024 / 1024))
    echo "  Created: ${actual_mb}MB file with approximately $lines_needed lines"
}

# Generate test files
generate_large_file "test_50mb.log" 50
generate_large_file "test_150mb.log" 150
generate_large_file "test_500mb.log" 500

echo
echo "Test files created in $TEST_DIR/"
echo

# Create a simple layout file
cat > "$TEST_DIR/test_layout.yaml" <<EOF
version: 1
groups:
  - id: performance_test
    name: "Performance Test Files"
    icon: "🚀"
    collapsed: false
    files:
      - path: "$TEST_DIR/test_50mb.log"
        name: "50MB Log"
      - path: "$TEST_DIR/test_150mb.log"
        name: "150MB Log"
      - path: "$TEST_DIR/test_500mb.log"
        name: "500MB Log"
EOF

echo "Layout file created: $TEST_DIR/test_layout.yaml"
echo

# Function to append lines to files continuously
append_lines() {
    echo "Starting background process to append lines to test files..."
    while true; do
        for file in "$TEST_DIR"/*.log; do
            echo "[$(date --iso-8601=seconds)] INFO  [LiveUpdate] New event at $(date +%s%N), random=${RANDOM}" >> "$file"
        done
        sleep 0.1  # Append lines every 100ms
    done
}

# Start appending in background
append_lines &
APPEND_PID=$!

echo "Background process started (PID: $APPEND_PID)"
echo
echo "To test vis-grep performance:"
echo "  ./target/release/vis-grep --tail-layout $TEST_DIR/test_layout.yaml"
echo
echo "Watch for:"
echo "  - Initial load time"
echo "  - UI responsiveness when switching between files"
echo "  - CPU/memory usage with large files"
echo "  - Scrolling performance in preview pane"
echo
echo "Press Ctrl+C to stop the background appender and cleanup"

# Cleanup on exit
cleanup() {
    echo
    echo "Stopping background process..."
    kill $APPEND_PID 2>/dev/null
    echo "To remove test files: rm -rf $TEST_DIR"
}

trap cleanup EXIT

# Wait for user to press Ctrl+C
while true; do
    sleep 1
done