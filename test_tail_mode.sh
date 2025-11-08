#!/bin/bash
# Test harness for vis-grep tail mode
# Starts log generators and launches vis-grep in tail mode

set -e

# Default parameters
NUM_FILES=3
RATE="medium"
DURATION=0  # 0 = infinite

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --files|-f)
            NUM_FILES="$2"
            shift 2
            ;;
        --rate|-r)
            RATE="$2"
            shift 2
            ;;
        --duration|-d)
            DURATION="$2"
            shift 2
            ;;
        --help|-h)
            echo "Test harness for vis-grep tail mode"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --files, -f NUM      Number of log files to generate (default: 3)"
            echo "  --rate, -r RATE      Rate: slow, medium, fast, burst (default: medium)"
            echo "  --duration, -d SEC   Duration in seconds, 0=infinite (default: 0)"
            echo "  --help, -h           Show this help"
            echo ""
            echo "Examples:"
            echo "  $0                             # 3 files, medium rate, infinite"
            echo "  $0 --files 5 --rate fast       # 5 files, fast rate"
            echo "  $0 --files 2 --duration 60     # 2 files, medium rate, 60 seconds"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "========================================="
echo "VisGrep Tail Mode Test Harness"
echo "========================================="
echo ""
echo "Configuration:"
echo "  Files: $NUM_FILES"
echo "  Rate: $RATE"
echo "  Duration: $([ $DURATION -eq 0 ] && echo 'infinite' || echo "${DURATION}s")"
echo ""

# Clean up old test logs
if [ -d test_logs ]; then
    echo "Cleaning up old test logs..."
    rm -f test_logs/test_*.log
fi

# Build vis-grep if needed
if [ ! -f target/release/vis-grep ]; then
    echo "Building vis-grep..."
    cargo build --release
    echo ""
fi

# Start log generator in background
echo "Starting log generator..."
python3 generate_test_logs.py \
    --files "$NUM_FILES" \
    --rate "$RATE" \
    --duration "$DURATION" \
    --dir test_logs &

LOG_GEN_PID=$!

# Give the generator a moment to create files
sleep 2

# Build the file list
FILE_LIST=""
for i in $(seq 1 $NUM_FILES); do
    FILE_LIST="$FILE_LIST test_logs/test_$i.log"
done

echo ""
echo "Starting vis-grep in tail mode..."
echo "Command: ./run.sh -f $FILE_LIST"
echo ""
echo "========================================="
echo ""

# Trap Ctrl+C to clean up
trap "echo ''; echo 'Stopping log generator...'; kill $LOG_GEN_PID 2>/dev/null; wait $LOG_GEN_PID 2>/dev/null; echo 'Done.'; exit 0" INT TERM

# Launch vis-grep in tail mode
./run.sh -f $FILE_LIST

# If vis-grep exits, stop the generator
kill $LOG_GEN_PID 2>/dev/null || true
wait $LOG_GEN_PID 2>/dev/null || true

echo ""
echo "Test harness complete."
