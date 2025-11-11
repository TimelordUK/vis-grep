# Windows Testing Guide

This document explains how to test vis-grep on Windows using the equivalent testing scripts.

## Test Scripts

The `test_tail_tree` script creates a realistic testing environment with:
- YAML layout file with groups of log files
- Initial log files with content
- Background process that continuously appends realistic log entries
- Automatic launch of vis-grep with the layout

### Original (Linux/Mac)
```bash
./test_tail_tree.sh 12 3 true
```

### PowerShell (Windows)
```powershell
.\test_tail_tree.ps1 -NumFiles 12 -NumGroups 3 -Nested $true
```

Or with positional arguments:
```powershell
.\test_tail_tree.ps1 12 3 $true
```

### Python (Cross-platform)
```bash
python test_tail_tree.py 12 3 true
```

## Parameters

All three versions accept the same parameters:

1. **Number of Files** (default: 10)
   - How many log files to create and monitor

2. **Number of Groups** (default: 2)
   - How many groups to organize files into

3. **Nested** (default: false)
   - Whether to create nested group structures
   - Use `$true`/`$false` in PowerShell
   - Use `true`/`false` in bash/Python

## Examples

### Simple test with 10 files in 2 groups
```powershell
.\test_tail_tree.ps1
```

### Complex nested layout
```powershell
.\test_tail_tree.ps1 -NumFiles 20 -NumGroups 4 -Nested $true
```

### Quick test with Python
```bash
python test_tail_tree.py 5 1 false
```

## What It Does

1. **Creates** `test_logs/` directory
2. **Generates** `test_tree_layout.yaml` with your specified structure
3. **Creates** initial log files (test_1.log, test_2.log, etc.)
4. **Starts** background process appending realistic log entries
5. **Launches** vis-grep with the layout file
6. **Stops** gracefully on Ctrl+C

## Output

You'll see colored output showing:
- ✓ Created layout file
- ✓ Created N log files
- Background log appender status
- vis-grep launching

## Generated Layout Example

```yaml
name: "Test Tree Layout - 3 groups, 12 files"
version: 1
settings:
  poll_interval_ms: 250
  auto_expand_active: true

groups:
  - name: "Application Logs"
    icon: "📱"
    collapsed: false
    groups:
      - name: "Core Services"
        files:
          - path: "C:/path/to/test_logs/test_1.log"
            name: "Test Log 1"
          - path: "C:/path/to/test_logs/test_2.log"
            name: "Test Log 2"
      - name: "Background Jobs"
        collapsed: true
        files:
          - path: "C:/path/to/test_logs/test_3.log"
            name: "Test Log 3"
          - path: "C:/path/to/test_logs/test_4.log"
            name: "Test Log 4"
```

## Requirements

### PowerShell
- Windows PowerShell 5.1+ or PowerShell Core 7+
- No additional dependencies

### Python
- Python 3.6+
- No external packages required (uses only standard library)

### Both
- Rust/Cargo installed
- vis-grep project in current directory

## Troubleshooting

### PowerShell Execution Policy
If you get an execution policy error:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Python Not Found
Make sure Python is in your PATH, or use full path:
```bash
C:\Python39\python.exe test_tail_tree.py
```

### Colors Not Showing
- PowerShell: Colors should work by default
- Python: Install `colorama` for better Windows color support (optional)

### Stopping the Script
Press **Ctrl+C** to stop both the log appender and vis-grep gracefully.

## Tips

- Start with small numbers for testing (5-10 files)
- Use nested layouts to test tree collapse/expand
- Watch the files being updated in real-time in vis-grep
- Check `test_logs/` directory to see the generated files
- Rerun with different parameters to test various layouts

