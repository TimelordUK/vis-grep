#!/bin/bash

# Test script for tail tree layout - generates YAML layout and test log files
# Usage: ./test_tail_tree.sh [files] [groups] [nested]
# Example: ./test_tail_tree.sh 10 2 true

# Parse arguments
NUM_FILES=${1:-10}
NUM_GROUPS=${2:-2}
NESTED=${3:-false}
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

# Build and run vis-grep
cargo build --release 2>/dev/null || cargo build

# Force X11 backend for WSL compatibility
unset WAYLAND_DISPLAY
export WINIT_UNIX_BACKEND=x11

if [ -f target/release/vis-grep ]; then
    ./target/release/vis-grep --tail-layout "$LAYOUT_FILE"
else
    ./target/debug/vis-grep --tail-layout "$LAYOUT_FILE"
fi