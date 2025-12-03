#!/bin/bash

# Create a test file with many lines
echo "Creating test file with 1000 lines..."
rm -f /tmp/test_large.log
for i in {1..1000}; do
    echo "[2025-12-03 19:00:00] INFO Line $i: This is a test log line with some content to make it realistic" >> /tmp/test_large.log
done

echo "Starting vis-grep in tail mode..."
echo "Press Ctrl+Shift+D to toggle debug overlay and see memory stats"
echo ""

# Run vis-grep with the test file
cargo run -- tail /tmp/test_large.log