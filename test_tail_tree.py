#!/usr/bin/env python3
"""
Test script for tail tree layout - generates YAML layout and test log files
Usage: python test_tail_tree.py [files] [groups] [nested]
Example: python test_tail_tree.py 10 2 true
"""

import sys
import os
import time
import random
import subprocess
import signal
import threading
from datetime import datetime
from pathlib import Path

# Parse arguments
NUM_FILES = int(sys.argv[1]) if len(sys.argv) > 1 else 10
NUM_GROUPS = int(sys.argv[2]) if len(sys.argv) > 2 else 2
NESTED = sys.argv[3].lower() == 'true' if len(sys.argv) > 3 else False

LOG_DIR = "test_logs"
LAYOUT_FILE = "test_tree_layout.yaml"

# Colors for terminal output
class Colors:
    GREEN = '\033[0;32m'
    BLUE = '\033[0;34m'
    YELLOW = '\033[1;33m'
    CYAN = '\033[0;36m'
    NC = '\033[0m'  # No Color
    
    @staticmethod
    def use_colors():
        return sys.platform != 'win32' or 'ANSICON' in os.environ

colors = Colors()
if not colors.use_colors():
    # Disable colors on Windows without ANSI support
    colors.GREEN = colors.BLUE = colors.YELLOW = colors.CYAN = colors.NC = ''

print(f"{colors.CYAN}Setting up tail tree test with:{colors.NC}")
print(f"  Files: {colors.GREEN}{NUM_FILES}{colors.NC}")
print(f"  Groups: {colors.GREEN}{NUM_GROUPS}{colors.NC}")
print(f"  Nested: {colors.GREEN}{NESTED}{colors.NC}")

# Create log directory
Path(LOG_DIR).mkdir(exist_ok=True)

def create_log_file(num):
    """Create a log file with initial content"""
    file_path = Path(LOG_DIR) / f"test_{num}.log"
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(f"[{timestamp}] Starting test log {num}\n")
        f.write(f"[{timestamp}] Initial content for testing\n")

def generate_group(group_num, indent, files_per_group, start_file):
    """Generate a group with files"""
    group_names = ["Application Logs", "System Logs", "Service Logs", "Database Logs",
                   "Network Logs", "Security Logs", "Performance Logs", "Error Logs"]
    icons = ["📱", "🖥️", "⚙️", "🗄️", "🌐", "🔒", "📊", "❌"]
    
    group_name = group_names[group_num % len(group_names)]
    icon = icons[group_num % len(icons)]
    collapsed = "true" if group_num > 1 else "false"
    
    output = f"""{indent}- name: "{group_name}"
{indent}  icon: "{icon}"
{indent}  collapsed: {collapsed}
"""
    
    # Add nested groups if requested
    if NESTED and group_num == 0:
        output += f"""{indent}  groups:
{indent}    - name: "Core Services"
{indent}      files:
"""
        # Add half the files to nested group
        nested_files = files_per_group // 2
        for j in range(nested_files):
            file_num = start_file + j
            # Use as_posix() to get forward slashes for YAML compatibility on Windows
            file_path = (Path.cwd() / LOG_DIR / f"test_{file_num}.log").absolute().as_posix()
            output += f"""{indent}        - path: "{file_path}"
{indent}          name: "Test Log {file_num}"
"""
            create_log_file(file_num)
        
        output += f"""{indent}    - name: "Background Jobs"
{indent}      collapsed: true
{indent}      files:
"""
        # Add remaining files to second nested group
        for j in range(nested_files, files_per_group):
            file_num = start_file + j
            # Use as_posix() to get forward slashes for YAML compatibility on Windows
            file_path = (Path.cwd() / LOG_DIR / f"test_{file_num}.log").absolute().as_posix()
            output += f"""{indent}        - path: "{file_path}"
{indent}          name: "Test Log {file_num}"
"""
            create_log_file(file_num)
    else:
        # Simple flat files
        output += f"""{indent}  files:
"""
        for j in range(files_per_group):
            file_num = start_file + j
            # Use as_posix() to get forward slashes for YAML compatibility on Windows
            file_path = (Path.cwd() / LOG_DIR / f"test_{file_num}.log").absolute().as_posix()
            output += f"""{indent}    - path: "{file_path}"
{indent}      name: "Test Log {file_num}"
"""
            create_log_file(file_num)
    
    return output

# Start building YAML
yaml_content = f"""name: "Test Tree Layout - {NUM_GROUPS} groups, {NUM_FILES} files"
version: 1
settings:
  poll_interval_ms: 250
  auto_expand_active: true

groups:
"""

# Calculate files per group
files_per_group = NUM_FILES // NUM_GROUPS
remainder = NUM_FILES % NUM_GROUPS

# Generate groups
file_counter = 1
for i in range(NUM_GROUPS):
    # Distribute remainder files across first groups
    files_in_group = files_per_group + (1 if i < remainder else 0)
    
    yaml_content += generate_group(i, "  ", files_in_group, file_counter)
    file_counter += files_in_group

# Write YAML file
with open(LAYOUT_FILE, 'w', encoding='utf-8') as f:
    f.write(yaml_content)

print(f"\n{colors.GREEN}✓ Created layout file: {LAYOUT_FILE}{colors.NC}")
print(f"{colors.GREEN}✓ Created {NUM_FILES} log files in {LOG_DIR}/{colors.NC}")

# Global flag for log appender
should_stop = False

def append_logs():
    """Continuously append random log entries to files"""
    messages = [
        "Processing request from client",
        "Database connection established",
        "Cache hit for key",
        "Starting background job",
        "Completed transaction",
        "Warning: High memory usage detected",
        "Error: Connection timeout",
        "Info: Service health check passed",
        "Debug: Query execution time",
        "Metric: Response time",
    ]
    
    levels = ["INFO", "WARN", "ERROR", "DEBUG"]
    
    while not should_stop:
        # Randomly select a few files to update
        files_to_update = random.randint(1, 3)
        
        for _ in range(files_to_update):
            file_num = random.randint(1, NUM_FILES)
            file_path = Path(LOG_DIR) / f"test_{file_num}.log"
            level = random.choice(levels)
            msg = random.choice(messages)
            timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
            value = random.randint(0, 999)
            
            with open(file_path, 'a', encoding='utf-8') as f:
                f.write(f"[{timestamp}] [{level}] {msg} {value}\n")
        
        time.sleep(random.uniform(0.1, 1.0))

# Start log appender in background thread
print(f"\n{colors.YELLOW}Starting log appender in background...{colors.NC}")
appender_thread = threading.Thread(target=append_logs, daemon=True)
appender_thread.start()

def cleanup(signum=None, frame=None):
    """Cleanup on exit"""
    global should_stop
    print(f"\n{colors.YELLOW}Stopping log appender...{colors.NC}")
    should_stop = True
    if appender_thread.is_alive():
        appender_thread.join(timeout=2)
    sys.exit(0)

# Register cleanup on signals
signal.signal(signal.SIGINT, cleanup)
signal.signal(signal.SIGTERM, cleanup)

# Launch vis-grep with the layout
print(f"\n{colors.CYAN}Launching vis-grep with tree layout...{colors.NC}")
print(f"{colors.CYAN}Press Ctrl+C to stop{colors.NC}\n")

# Build and run vis-grep
try:
    print("Building vis-grep...", file=sys.stderr)
    
    # Try release build first
    result = subprocess.run(["cargo", "build", "--release"],
                          capture_output=True, text=True)
    
    if result.returncode == 0:
        exe_path = Path("target/release/vis-grep.exe" if sys.platform == "win32" else "target/release/vis-grep")
    else:
        # Fall back to debug build
        subprocess.run(["cargo", "build"], capture_output=True)
        exe_path = Path("target/debug/vis-grep.exe" if sys.platform == "win32" else "target/debug/vis-grep")
    
    if exe_path.exists():
        # Run vis-grep with layout file
        subprocess.run([str(exe_path), "--tail-layout", LAYOUT_FILE])
    else:
        print("Error: Could not find vis-grep executable", file=sys.stderr)
        sys.exit(1)
        
except KeyboardInterrupt:
    pass
finally:
    cleanup()

