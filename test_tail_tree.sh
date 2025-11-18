#!/bin/bash

# Test script for tail tree layout - generates YAML layout and test log files
# Usage: ./test_tail_tree.sh [files] [groups] [nested] [release]
# Example: ./test_tail_tree.sh 10 2 true release

# Check for help flag
if [[ "$1" == "-h" || "$1" == "--help" ]]; then
    echo "Usage: $0 [files] [groups] [nested] [mode]"
    echo
    echo "Arguments:"
    echo "  files    Number of log files to create (default: 10)"
    echo "  groups   Number of groups in the tree (default: 2)"
    echo "  nested   Whether to create nested groups: true/false (default: false)"
    echo "  mode     Build mode: debug/release (default: debug)"
    echo
    echo "Examples:"
    echo "  $0                    # 10 files, 2 groups, flat, debug mode"
    echo "  $0 20 3               # 20 files, 3 groups, flat, debug mode"
    echo "  $0 15 2 true          # 15 files, 2 groups, nested, debug mode"
    echo "  $0 8 2 true release   # 8 files, 2 groups, nested, release mode (minimal logging)"
    echo
    echo "Environment variables:"
    echo "  RUST_LOG    Set log level (default: 'debug' for debug mode, 'info' for release)"
    echo "              Example: RUST_LOG=warn $0 10 2 false release"
    exit 0
fi

# Parse arguments
NUM_FILES=${1:-10}
NUM_GROUPS=${2:-2}
NESTED=${3:-false}
BUILD_MODE=${4:-debug}  # 'debug' or 'release'
LOG_DIR="test_logs"
LAYOUT_FILE="test_tree_layout.yaml"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}Setting up tail tree test with:${NC}"
echo -e "  Files: ${GREEN}$NUM_FILES${NC}"
echo -e "  Groups: ${GREEN}$NUM_GROUPS${NC}"
echo -e "  Nested: ${GREEN}$NESTED${NC}"
echo -e "  Build: ${GREEN}$BUILD_MODE${NC}"

# Create log directory
mkdir -p "$LOG_DIR"

# Start building YAML
cat > "$LAYOUT_FILE" << EOF
name: "Test Tree Layout - ${NUM_GROUPS} groups, ${NUM_FILES} files"
version: 1
settings:
  poll_interval_ms: 250
  auto_expand_active: true

groups:
EOF

# Function to generate a group with files
generate_group() {
    local group_num=$1
    local indent=$2
    local files_per_group=$3
    local start_file=$4
    
    # Group names based on common log scenarios
    local group_names=("Application Logs" "System Logs" "Service Logs" "Database Logs" "Network Logs" "Security Logs" "Performance Logs" "Error Logs")
    local icons=("📱" "🖥️" "⚙️" "🗄️" "🌐" "🔒" "📊" "❌")
    
    local group_name="${group_names[$((group_num % ${#group_names[@]}))]}"
    local icon="${icons[$((group_num % ${#icons[@]}))]}"
    
    cat >> "$LAYOUT_FILE" << EOF
${indent}- name: "$group_name"
${indent}  icon: "$icon"
${indent}  collapsed: $([ $group_num -gt 1 ] && echo "true" || echo "false")
EOF

    # Add nested groups if requested
    if [[ "$NESTED" == "true" && $group_num -eq 0 ]]; then
        cat >> "$LAYOUT_FILE" << EOF
${indent}  groups:
${indent}    - name: "Core Services"
${indent}      files:
EOF
        # Add half the files to nested group
        local nested_files=$((files_per_group / 2))
        for (( j=0; j<nested_files; j++ )); do
            local file_num=$((start_file + j))
            local filename=$(get_log_filename $file_num)
            cat >> "$LAYOUT_FILE" << EOF
${indent}        - path: "$(pwd)/$LOG_DIR/$filename"
EOF
            # Create the log file
            create_log_file $file_num
        done
        
        cat >> "$LAYOUT_FILE" << EOF
${indent}    - name: "Background Jobs"
${indent}      collapsed: true
${indent}      files:
EOF
        # Add remaining files to second nested group
        for (( j=nested_files; j<files_per_group; j++ )); do
            local file_num=$((start_file + j))
            local filename=$(get_log_filename $file_num)
            cat >> "$LAYOUT_FILE" << EOF
${indent}        - path: "$(pwd)/$LOG_DIR/$filename"
EOF
            # Create the log file
            create_log_file $file_num
        done
    else
        # Simple flat files
        cat >> "$LAYOUT_FILE" << EOF
${indent}  files:
EOF
        for (( j=0; j<files_per_group; j++ )); do
            local file_num=$((start_file + j))
            local filename=$(get_log_filename $file_num)
            cat >> "$LAYOUT_FILE" << EOF
${indent}    - path: "$(pwd)/$LOG_DIR/$filename"
EOF
            # Create the log file
            create_log_file $file_num
        done
    fi
}

# Realistic log file names of varying lengths
get_log_filename() {
    local num=$1
    local names=(
        "API.log"
        "BookingEngine.log"
        "Extractor.log"
        "PaymentProcessor.log"
        "Auth.log"
        "NotificationService.log"
        "DB.log"
        "Cache.log"
        "MessageQueue.log"
        "WebServer.log"
        "BackgroundWorker.log"
        "Scheduler.log"
        "EmailService.log"
        "FileUpload.log"
        "ReportGenerator.log"
        "Analytics.log"
        "AuditLog.log"
        "Session.log"
        "SecurityMonitor.log"
        "HealthCheck.log"
    )
    echo "${names[$((num % ${#names[@]}))]}"
}

# Function to create a log file with initial content
create_log_file() {
    local num=$1
    local filename=$(get_log_filename $num)
    local file="$LOG_DIR/$filename"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Starting $filename" > "$file"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] Initial content for testing" >> "$file"
    echo "$filename"  # Return the filename for use in YAML
}

# Calculate files per group
FILES_PER_GROUP=$((NUM_FILES / NUM_GROUPS))
REMAINDER=$((NUM_FILES % NUM_GROUPS))

# Generate groups
file_counter=1
for (( i=0; i<NUM_GROUPS; i++ )); do
    # Distribute remainder files across first groups
    if [ $i -lt $REMAINDER ]; then
        files_in_group=$((FILES_PER_GROUP + 1))
    else
        files_in_group=$FILES_PER_GROUP
    fi
    
    generate_group $i "  " $files_in_group $file_counter
    file_counter=$((file_counter + files_in_group))
done

echo -e "\n${GREEN}✓ Created layout file: $LAYOUT_FILE${NC}"
echo -e "${GREEN}✓ Created $NUM_FILES log files in $LOG_DIR/${NC}"

# Function to append random log entries to files
append_logs() {
    local messages=(
        "Processing request from client"
        "Database connection established"
        "Cache hit for key"
        "Starting background job"
        "Completed transaction"
        "Warning: High memory usage detected"
        "Error: Connection timeout"
        "Info: Service health check passed"
        "Debug: Query execution time"
        "Metric: Response time"
    )
    
    local levels=("INFO" "WARN" "ERROR" "DEBUG")
    
    while true; do
        # Randomly select a few files to update
        local files_to_update=$((RANDOM % 3 + 1))

        for (( i=0; i<files_to_update; i++ )); do
            local file_num=$((RANDOM % NUM_FILES))
            local filename=$(get_log_filename $file_num)
            local file="$LOG_DIR/$filename"
            local level="${levels[$((RANDOM % ${#levels[@]}))]}"
            local msg="${messages[$((RANDOM % ${#messages[@]}))]}"
            local timestamp=$(date '+%Y-%m-%d %H:%M:%S.%3N')

            echo "[$timestamp] [$level] $msg $((RANDOM % 1000))" >> "$file"
        done
        
        sleep 0.$((RANDOM % 1000))  # Random sleep 0-1 second
    done
}

# Start log appender in background
echo -e "\n${YELLOW}Starting log appender in background...${NC}"
append_logs &
APPENDER_PID=$!

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Stopping log appender...${NC}"
    kill $APPENDER_PID 2>/dev/null
    exit 0
}

trap cleanup EXIT INT TERM

# Launch vis-grep with the layout
echo -e "\n${BLUE}Launching vis-grep with tree layout...${NC}"
echo -e "${BLUE}Press Ctrl+C to stop${NC}\n"

# Build vis-grep based on mode
if [[ "$BUILD_MODE" == "release" ]]; then
    echo -e "${YELLOW}Building in release mode...${NC}"
    cargo build --release 2>&1 | tail -3
    
    # Set default log level for release mode (info instead of debug)
    DEFAULT_LOG_LEVEL="info"
    BINARY_PATH="./target/release/vis-grep"
else
    echo -e "${YELLOW}Building in debug mode...${NC}"
    cargo build 2>&1 | tail -3
    
    # Set default log level for debug mode
    DEFAULT_LOG_LEVEL="debug"
    BINARY_PATH="./target/debug/vis-grep"
fi

# Run vis-grep directly or via run.sh
if [[ "$BUILD_MODE" == "release" && -f "$BINARY_PATH" ]]; then
    # Run release binary directly with minimal logging
    echo -e "${GREEN}Running release build with log level: ${RUST_LOG:-$DEFAULT_LOG_LEVEL}${NC}"
    RUST_LOG="${RUST_LOG:-$DEFAULT_LOG_LEVEL}" "$BINARY_PATH" --tail-layout "$LAYOUT_FILE"
else
    # Run via run.sh (respects RUST_LOG env var)
    echo -e "${GREEN}Running debug build with log level: ${RUST_LOG:-$DEFAULT_LOG_LEVEL}${NC}"
    RUST_LOG="${RUST_LOG:-$DEFAULT_LOG_LEVEL}" ./run.sh --tail-layout "$LAYOUT_FILE"
fi