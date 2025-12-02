# Memory Testing for vis-grep Tail Mode

This directory contains tools to test and profile memory usage in vis-grep's tail mode without the GUI overhead.

## Files

- `headless_tail_standalone.rs` - Standalone Rust implementation of tail mode core functionality
- `generate_logs.py` - Python script to generate test log data at configurable rates
- `profile_memory.py` - Advanced memory profiler with graphing capabilities
- `build_headless.sh` - Build script for the standalone test
- `run_headless_test.sh` - Simple test runner
- `run_memory_test.sh` - Comprehensive test with multiple files

## Quick Start

1. Build the headless test:
   ```bash
   ./build_headless.sh
   ```

2. Run a simple test:
   ```bash
   ./run_headless_test.sh
   ```

3. Run with custom parameters:
   ```bash
   ../target/release/headless_tail file1.log file2.log --buffer-size 5000 --duration 120
   ```

## Memory Profile Analysis

To create a detailed memory profile with graphs:

```bash
python3 profile_memory.py ../target/release/headless_tail test.log --duration 60
```

This will generate:
- `memory_profile_YYYYMMDD_HHMMSS.png` - Visual graph of memory usage
- `memory_data_YYYYMMDD_HHMMSS.csv` - Raw data for further analysis

## Key Findings

The headless test replicates these core vis-grep features:
- File watching with polling
- String interning cache (10,000 entries max)
- Ring buffer with configurable size
- Log level detection
- Aggressive buffer capacity management

This allows us to isolate memory issues from GUI overhead and identify if the growth is from:
1. String allocation patterns
2. Buffer management
3. File watching overhead
4. Cache growth

## Log Generator Options

Generate different types of logs:

```bash
# High volume, mixed size logs
python3 generate_logs.py file.log --rate 200 --burst --template mixed

# Consistent small logs
python3 generate_logs.py file.log --rate 50 --template short

# Large log entries
python3 generate_logs.py file.log --rate 10 --template huge
```

## Memory Optimization Strategies

Based on testing, consider:
1. **Line length limits** - Truncate very long lines before interning
2. **Cache eviction** - LRU or time-based eviction for string cache
3. **Filtered buffer** - Don't store filtered-out lines
4. **Memory pressure response** - Reduce buffers when memory is high