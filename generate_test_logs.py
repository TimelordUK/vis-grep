#!/usr/bin/env python3
"""
Test log generator for VisGrep tail mode testing.

Generates multiple log files with realistic content at variable rates
to simulate growing log files for testing the tail -f functionality.

Usage:
    python3 generate_test_logs.py --files 3 --duration 60
    python3 generate_test_logs.py --files 5 --rate slow --dir test_logs
"""

import argparse
import random
import time
import threading
import sys
from datetime import datetime
from pathlib import Path


# Sample log patterns
LOG_LEVELS = ["DEBUG", "INFO", "WARN", "ERROR", "FATAL"]
LOG_COMPONENTS = [
    "Database", "WebServer", "APIHandler", "AuthService",
    "CacheManager", "MessageQueue", "FileProcessor", "Scheduler"
]

# Lorem ipsum-style words for message generation
IPSUM_WORDS = [
    "process", "initialize", "complete", "failed", "success", "timeout",
    "connection", "request", "response", "data", "query", "transaction",
    "session", "token", "validation", "configuration", "parameter",
    "thread", "pool", "buffer", "cache", "memory", "disk", "network",
    "latency", "throughput", "bandwidth", "load", "balance", "cluster",
    "node", "replica", "primary", "secondary", "master", "worker"
]

# Realistic log messages templates
MESSAGE_TEMPLATES = [
    "Processing {noun} for user {user_id}",
    "Database query took {latency}ms",
    "API endpoint /{endpoint} returned status {status}",
    "Cache hit ratio: {percentage}%",
    "Connection pool size: {count}/{max_count}",
    "{action} {noun} completed successfully",
    "Failed to {action} {noun}: {error}",
    "Memory usage: {percentage}% ({size}MB / {max_size}MB)",
    "Received {count} messages from queue",
    "Background job '{job_name}' {status}",
    "User {user_id} authenticated via {method}",
    "File {filename} processed in {latency}ms",
    "Request timeout after {timeout}s",
    "Retry attempt {attempt}/{max_attempts}",
    "{count} active sessions, {idle} idle",
]

NOUNS = ["record", "document", "entity", "object", "resource", "item", "entry"]
ACTIONS = ["create", "update", "delete", "fetch", "process", "validate", "parse"]
ERRORS = ["timeout", "connection refused", "invalid format", "not found", "permission denied"]
METHODS = ["OAuth2", "SAML", "JWT", "API Key", "Basic Auth"]
JOB_NAMES = ["data_sync", "report_generation", "cleanup", "backup", "indexing"]
ENDPOINTS = ["users", "orders", "products", "auth", "analytics", "search"]
FILENAMES = ["data.csv", "report.pdf", "config.json", "backup.tar.gz", "log.txt"]


def generate_log_line():
    """Generate a realistic log line."""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S.%f")[:-3]
    level = random.choice(LOG_LEVELS)
    component = random.choice(LOG_COMPONENTS)

    # Weight towards INFO/DEBUG (more common in real logs)
    weights = [0.3, 0.5, 0.12, 0.06, 0.02]  # DEBUG, INFO, WARN, ERROR, FATAL
    level = random.choices(LOG_LEVELS, weights=weights)[0]

    # Generate message from template
    template = random.choice(MESSAGE_TEMPLATES)
    message = template.format(
        noun=random.choice(NOUNS),
        action=random.choice(ACTIONS),
        user_id=f"user_{random.randint(1000, 9999)}",
        latency=random.randint(10, 2000),
        status=random.choice([200, 201, 400, 404, 500, 503]),
        percentage=random.randint(10, 99),
        count=random.randint(1, 100),
        max_count=random.randint(100, 500),
        error=random.choice(ERRORS),
        method=random.choice(METHODS),
        job_name=random.choice(JOB_NAMES),
        endpoint=random.choice(ENDPOINTS),
        filename=random.choice(FILENAMES),
        timeout=random.randint(5, 60),
        attempt=random.randint(1, 3),
        max_attempts=3,
        idle=random.randint(0, 50),
        size=random.randint(100, 8000),
        max_size=random.randint(8000, 16000),
    )

    return f"{timestamp} [{level:5}] {component:15} - {message}\n"


class LogGenerator:
    """Generates log entries for a single file."""

    def __init__(self, filepath, rate_config, file_index):
        self.filepath = filepath
        self.rate_config = rate_config
        self.file_index = file_index
        self.running = False
        self.thread = None
        self.lines_written = 0

    def start(self):
        """Start generating logs in a background thread."""
        self.running = True
        self.thread = threading.Thread(target=self._generate_loop, daemon=True)
        self.thread.start()

    def stop(self):
        """Stop generating logs."""
        self.running = False
        if self.thread:
            self.thread.join()

    def _generate_loop(self):
        """Main loop that generates log entries."""
        with open(self.filepath, 'a') as f:
            while self.running:
                # Random interval based on rate
                interval = random.uniform(
                    self.rate_config['min_interval'],
                    self.rate_config['max_interval']
                )

                # Sometimes write bursts
                if random.random() < 0.1:  # 10% chance of burst
                    burst_size = random.randint(3, 10)
                    for _ in range(burst_size):
                        line = generate_log_line()
                        f.write(line)
                        f.flush()
                        self.lines_written += 1
                    print(f"[File {self.file_index}] Burst: {burst_size} lines")
                else:
                    # Single line
                    line = generate_log_line()
                    f.write(line)
                    f.flush()
                    self.lines_written += 1

                time.sleep(interval)


def main():
    parser = argparse.ArgumentParser(
        description="Generate test log files with growing content",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Generate 3 files at medium speed for 60 seconds
  python3 generate_test_logs.py --files 3 --duration 60

  # Generate 5 files at fast rate indefinitely
  python3 generate_test_logs.py --files 5 --rate fast --duration 0

  # Generate 2 files with custom rate (0.5-2 second intervals)
  python3 generate_test_logs.py --files 2 --min-interval 0.5 --max-interval 2.0
        """
    )

    parser.add_argument(
        '--files', '-f',
        type=int,
        default=3,
        help='Number of log files to generate (default: 3)'
    )

    parser.add_argument(
        '--duration', '-d',
        type=int,
        default=60,
        help='Duration in seconds (0 for infinite, default: 60)'
    )

    parser.add_argument(
        '--dir',
        type=str,
        default='test_logs',
        help='Directory to create log files in (default: test_logs)'
    )

    parser.add_argument(
        '--rate',
        choices=['slow', 'medium', 'fast', 'burst'],
        default='medium',
        help='Preset rate: slow (2-5s), medium (0.5-2s), fast (0.1-0.5s), burst (0.01-0.1s)'
    )

    parser.add_argument(
        '--min-interval',
        type=float,
        help='Minimum interval between log lines in seconds (overrides --rate)'
    )

    parser.add_argument(
        '--max-interval',
        type=float,
        help='Maximum interval between log lines in seconds (overrides --rate)'
    )

    parser.add_argument(
        '--prefix',
        type=str,
        default='test',
        help='Prefix for log filenames (default: test)'
    )

    args = parser.parse_args()

    # Determine rate configuration
    rate_presets = {
        'slow': {'min_interval': 2.0, 'max_interval': 5.0},
        'medium': {'min_interval': 0.5, 'max_interval': 2.0},
        'fast': {'min_interval': 0.1, 'max_interval': 0.5},
        'burst': {'min_interval': 0.01, 'max_interval': 0.1},
    }

    if args.min_interval is not None and args.max_interval is not None:
        rate_config = {
            'min_interval': args.min_interval,
            'max_interval': args.max_interval
        }
    else:
        rate_config = rate_presets[args.rate]

    # Create output directory
    output_dir = Path(args.dir)
    output_dir.mkdir(exist_ok=True)

    print(f"VisGrep Test Log Generator")
    print(f"==========================")
    print(f"Files: {args.files}")
    print(f"Duration: {'infinite' if args.duration == 0 else f'{args.duration}s'}")
    print(f"Rate: {rate_config['min_interval']:.2f}s - {rate_config['max_interval']:.2f}s")
    print(f"Directory: {output_dir}")
    print()

    # Create log files
    generators = []
    for i in range(args.files):
        filepath = output_dir / f"{args.prefix}_{i+1}.log"

        # Create file with header
        with open(filepath, 'w') as f:
            f.write(f"# Log file generated at {datetime.now()}\n")
            f.write(f"# Rate: {rate_config['min_interval']}-{rate_config['max_interval']}s\n")
            f.write("# ============================================\n")

        generator = LogGenerator(filepath, rate_config, i+1)
        generators.append(generator)
        print(f"Created: {filepath}")

    print("\nStarting log generation... (Ctrl+C to stop)")
    print()

    # Start all generators
    for gen in generators:
        gen.start()

    try:
        # Run for specified duration or until interrupted
        if args.duration > 0:
            start_time = time.time()
            while time.time() - start_time < args.duration:
                time.sleep(1)
                elapsed = int(time.time() - start_time)
                remaining = args.duration - elapsed

                # Print status
                total_lines = sum(g.lines_written for g in generators)
                sys.stdout.write(f"\r[{elapsed:3d}s / {args.duration}s] Lines written: {total_lines:5d}  ")
                sys.stdout.flush()
        else:
            # Run indefinitely
            start_time = time.time()
            while True:
                time.sleep(1)
                elapsed = int(time.time() - start_time)
                total_lines = sum(g.lines_written for g in generators)
                sys.stdout.write(f"\r[{elapsed:4d}s] Lines written: {total_lines:6d}  ")
                sys.stdout.flush()

    except KeyboardInterrupt:
        print("\n\nStopping log generation...")

    # Stop all generators
    for gen in generators:
        gen.stop()

    print()
    print("Summary:")
    print("--------")
    for i, gen in enumerate(generators):
        filepath = output_dir / f"{args.prefix}_{i+1}.log"
        size = filepath.stat().st_size
        print(f"  {filepath.name}: {gen.lines_written} lines, {size:,} bytes")

    total_lines = sum(g.lines_written for g in generators)
    print(f"\nTotal: {total_lines} lines written")
    print(f"\nTest with: ./run.sh -f {' '.join(str(output_dir / f'{args.prefix}_{i+1}.log') for i in range(args.files))}")


if __name__ == '__main__':
    main()
