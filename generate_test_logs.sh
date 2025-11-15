#!/bin/bash

# Generate test log files with various log levels

OUTPUT_DIR="test_logs"
mkdir -p "$OUTPUT_DIR"

# Test file 1: Standard bracketed format
echo "Generating test logs in $OUTPUT_DIR/"

cat > "$OUTPUT_DIR/app.log" << 'EOF'
[TRACE] Application starting up
[DEBUG] Loading configuration from /etc/app/config.yaml
[INFO] Server listening on port 8080
[DEBUG] Initializing database connection pool
[INFO] Database connection established
[WARN] Connection pool size below recommended minimum
[DEBUG] Loading user preferences
[INFO] User 'admin' logged in from 192.168.1.100
[ERROR] Failed to connect to external API
[WARN] Retrying connection (attempt 1/3)
[ERROR] Failed to connect to external API
[WARN] Retrying connection (attempt 2/3)
[ERROR] Failed to connect to external API
[FATAL] Unable to establish API connection - shutting down
EOF

# Test file 2: Colon-separated format
cat > "$OUTPUT_DIR/service.log" << 'EOF'
TRACE: Entering function processRequest
DEBUG: Request headers: {"Content-Type": "application/json"}
INFO: Processing request from user 'john.doe'
DEBUG: Validating input parameters
INFO: Input validation successful
WARN: Rate limit approaching (80% of quota used)
DEBUG: Executing database query
INFO: Query returned 42 results
ERROR: Timeout while waiting for response from backend
WARN: Falling back to cached data
INFO: Request completed in 2.3s
EOF

# Test file 3: Angular bracket format
cat > "$OUTPUT_DIR/worker.log" << 'EOF'
<trace> Worker thread #1 started
<debug> Processing job ID: abc123
<info> Job type: data_export
<debug> Loading dataset from cache
<info> Dataset contains 1000 records
<warn> Memory usage at 75%
<debug> Applying filters
<info> Export completed successfully
<trace> Worker thread #1 idle
<debug> Cleaning up temporary files
EOF

# Test file 4: Short forms
cat > "$OUTPUT_DIR/debug.log" << 'EOF'
TRC Initializing module
DBG Setting up event handlers
INF Module initialized successfully
DBG Registering callbacks
INF Callbacks registered: 5
WRN Deprecated function call detected
DBG Processing event queue
INF Event queue empty
ERR Null pointer exception in handler
WRN Recovering from error state
INF Module recovered
DBG Cleanup complete
EOF

# Test file 5: Mixed formats and plain text
cat > "$OUTPUT_DIR/mixed.log" << 'EOF'
[INFO] System started
This is a plain text line without a log level
[DEBUG] Some debug information
Stack trace:
  at function1 (file.js:10)
  at function2 (file.js:20)
[ERROR] Error occurred
More details about the error
[WARN] Warning message
Plain text continuation
INFO: Another format
[INFO] Back to brackets
EOF

echo "Test logs generated in $OUTPUT_DIR/"
echo ""
echo "To test:"
echo "  1. Run: ./target/release/vis-grep --tail $OUTPUT_DIR/*.log"
echo "  2. Press 'L' to cycle through log levels (ALL -> INFO+ -> WARN+ -> ERROR -> ALL)"
echo "  3. Use the UI buttons to select specific log levels"
echo "  4. Toggle 'Show UNKNOWN' to see/hide plain text lines"
