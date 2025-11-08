# VisGrep Test Tools

Test harness for developing and testing the tail mode feature.

## Log Generator

`generate_test_logs.py` - Generates realistic, growing log files to simulate real-world log monitoring scenarios.

### Features

- **Multiple files**: Generate any number of log files simultaneously
- **Variable rates**: Configurable output rates (slow, medium, fast, burst)
- **Realistic content**: Log levels, timestamps, components, and messages
- **Burst simulation**: Random bursts of activity (like real applications)
- **Custom intervals**: Fine-tune min/max intervals between log lines

### Usage

```bash
# Quick start - 3 files for 60 seconds at medium rate
python3 generate_test_logs.py

# 5 files at fast rate, run indefinitely
python3 generate_test_logs.py --files 5 --rate fast --duration 0

# Custom rate: 0.5-2 second intervals
python3 generate_test_logs.py --min-interval 0.5 --max-interval 2.0

# Slow drip (like production logs)
python3 generate_test_logs.py --files 10 --rate slow --duration 300
```

### Rate Presets

- **slow**: 2-5 seconds between log lines (like quiet production logs)
- **medium**: 0.5-2 seconds (typical application logging)
- **fast**: 0.1-0.5 seconds (busy application)
- **burst**: 0.01-0.1 seconds (high-volume logging)

### Options

```
--files, -f NUM         Number of log files (default: 3)
--duration, -d SEC      Duration in seconds, 0=infinite (default: 60)
--rate PRESET           Rate preset: slow, medium, fast, burst
--min-interval SEC      Custom minimum interval
--max-interval SEC      Custom maximum interval
--dir PATH              Output directory (default: test_logs)
--prefix NAME           Filename prefix (default: test)
```

### Sample Output

```
2025-11-08 20:21:39.618 [INFO ] Scheduler       - Background job 'cleanup' completed
2025-11-08 20:21:39.954 [INFO ] MessageQueue    - Failed to process item: invalid format
2025-11-08 20:21:40.234 [INFO ] WebServer       - Received 10 messages from queue
2025-11-08 20:21:40.449 [WARN ] APIHandler      - User user_5863 authenticated via API Key
2025-11-08 20:21:40.900 [WARN ] APIHandler      - 73 active sessions, 31 idle
```

## Test Harness

`test_tail_mode.sh` - All-in-one test harness that starts the log generator and launches vis-grep in tail mode.

### Usage

```bash
# Start with defaults (3 files, medium rate)
./test_tail_mode.sh

# 5 files at fast rate
./test_tail_mode.sh --files 5 --rate fast

# Custom configuration
./test_tail_mode.sh --files 2 --rate slow --duration 120
```

### What It Does

1. Cleans up old test logs
2. Builds vis-grep (if needed)
3. Starts the log generator in the background
4. Launches vis-grep in tail mode with the generated files
5. Cleans up on exit

### Options

```
--files, -f NUM      Number of log files (default: 3)
--rate, -r RATE      Rate: slow, medium, fast, burst (default: medium)
--duration, -d SEC   Duration in seconds, 0=infinite (default: 0)
--help, -h           Show help
```

## Manual Testing

You can also test tail mode manually:

```bash
# Terminal 1: Start the log generator
python3 generate_test_logs.py --files 3 --rate medium --duration 0

# Terminal 2: Start vis-grep in tail mode
./run.sh -f test_logs/test_1.log test_logs/test_2.log test_logs/test_3.log
```

Or use the tail subcommand:

```bash
./run.sh tail test_logs/test_1.log test_logs/test_2.log test_logs/test_3.log
```

## Testing Scenarios

### Scenario 1: Low Activity Monitoring
Simulate monitoring quiet production logs:
```bash
./test_tail_mode.sh --files 10 --rate slow
```

### Scenario 2: Busy Application
Simulate a busy application with multiple log files:
```bash
./test_tail_mode.sh --files 5 --rate fast
```

### Scenario 3: High-Volume Logging
Simulate extreme logging scenarios:
```bash
./test_tail_mode.sh --files 3 --rate burst
```

### Scenario 4: Network Share Simulation
For testing the size-based file change detection (important for Windows SMB shares):
```bash
# Generate logs to a network-mounted directory
python3 generate_test_logs.py --dir /mnt/network/logs --files 5
```

## Features to Test

When implementing tail mode, test these scenarios:

- [x] Command-line argument parsing (`-f` flag, `tail` subcommand)
- [ ] Multiple files being monitored simultaneously
- [ ] Activity indicators (● active, ○ idle)
- [ ] Size-based file change detection
- [ ] Handling file rotation/truncation
- [ ] Filtering/search on live output
- [ ] Performance with many files (10+)
- [ ] Burst handling (sudden influx of log lines)
- [ ] Pause/resume functionality
- [ ] Auto-scroll to bottom

## Notes

- Log files are created in `test_logs/` directory (gitignored)
- Each file includes a header with generation timestamp and rate
- The generator supports random bursts (10% chance) to simulate real applications
- Press Ctrl+C to stop the generator or test harness cleanly
