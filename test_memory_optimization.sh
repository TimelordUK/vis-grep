#!/bin/bash
set -e

echo "Building vis-grep with optimizations..."
cargo build --release

echo -e "\nSetting up test environment..."
rm -rf test_memory_logs
mkdir -p test_memory_logs

# Create 10 test log files
for i in {0..9}; do
    touch test_memory_logs/test_$i.log
done

# Create a simple tree layout
cat > test_memory_tree.yaml << 'EOF'
version: 1
groups:
  - name: "Memory Test Logs"
    files:
      - test_memory_logs/test_0.log
      - test_memory_logs/test_1.log
      - test_memory_logs/test_2.log
      - test_memory_logs/test_3.log
      - test_memory_logs/test_4.log
      - test_memory_logs/test_5.log
      - test_memory_logs/test_6.log
      - test_memory_logs/test_7.log
      - test_memory_logs/test_8.log
      - test_memory_logs/test_9.log
EOF

echo -e "\nStarting log generator in background..."
# Generate logs continuously
(
    count=0
    while true; do
        for i in {0..9}; do
            # Write 100 lines to each file per cycle
            for j in {1..100}; do
                echo "[$(date '+%Y-%m-%d %H:%M:%S.%3N')] [INFO] Test log message $count - This is a typical log line with some content that might appear in real application logs. Request ID: $(uuidgen), User: test_user_$((count % 100)), Action: process_data, Duration: $((RANDOM % 1000))ms" >> test_memory_logs/test_$i.log
                ((count++))
            done
        done
        sleep 0.1
    done
) &

LOG_GEN_PID=$!

echo "Log generator PID: $LOG_GEN_PID"

# Function to get memory usage
get_memory() {
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        ps -o rss= -p $1 | awk '{print $1/1024}'
    else
        ps -o rss= -p $1 | awk '{print $1/1024}'
    fi
}

echo -e "\nStarting vis-grep with tail mode..."
./target/release/vis-grep tail --layout-file test_memory_tree.yaml &
VIS_GREP_PID=$!

echo "vis-grep PID: $VIS_GREP_PID"
echo -e "\nMonitoring memory usage for 60 seconds...\n"

# Monitor for 60 seconds
echo "Time(s) | Memory(MB) | Log Lines"
echo "--------|------------|----------"

for i in {1..60}; do
    if ! kill -0 $VIS_GREP_PID 2>/dev/null; then
        echo "vis-grep process died!"
        break
    fi
    
    MEM=$(get_memory $VIS_GREP_PID 2>/dev/null || echo "N/A")
    LINES=$(wc -l test_memory_logs/*.log | tail -1 | awk '{print $1}')
    printf "%7d | %10s | %9d\n" $i "$MEM" "$LINES"
    
    sleep 1
done

echo -e "\nCleaning up..."
kill $LOG_GEN_PID 2>/dev/null || true
kill $VIS_GREP_PID 2>/dev/null || true

echo "Test complete!"