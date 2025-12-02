#!/usr/bin/env python3
"""
Heavy log generator designed to stress test memory usage.
Generates logs with:
- More unique content (less deduplication benefit)
- Larger average size
- High cardinality data
"""

import argparse
import time
import random
import uuid
import json
import threading
from datetime import datetime

# Generate more unique content to stress the string cache
def generate_unique_id():
    return str(uuid.uuid4())

def generate_json_payload(size='medium'):
    if size == 'small':
        return json.dumps({
            'id': generate_unique_id(),
            'timestamp': time.time(),
            'value': random.randint(1, 1000)
        })
    elif size == 'medium':
        return json.dumps({
            'request_id': generate_unique_id(),
            'user_id': f'user_{random.randint(1, 1000000)}',
            'session_id': generate_unique_id(),
            'timestamp': time.time(),
            'path': f'/api/v2/{random.choice(["users", "orders", "products", "analytics"])}/{random.randint(1, 1000000)}',
            'params': {f'param_{i}': generate_unique_id() for i in range(5)},
            'headers': {
                'User-Agent': f'Mozilla/5.0 (Bot {random.randint(1, 1000)})',
                'X-Request-ID': generate_unique_id(),
                'X-Session-ID': generate_unique_id(),
            }
        })
    else:  # large
        return json.dumps({
            'transaction_id': generate_unique_id(),
            'timestamp': time.time(),
            'user': {
                'id': f'user_{random.randint(1, 1000000)}',
                'session': generate_unique_id(),
                'ip': f'{random.randint(1,255)}.{random.randint(1,255)}.{random.randint(1,255)}.{random.randint(1,255)}',
                'country': random.choice(['US', 'UK', 'DE', 'FR', 'JP', 'CN', 'IN', 'BR']),
            },
            'items': [
                {
                    'id': generate_unique_id(),
                    'product_id': f'prod_{random.randint(1, 100000)}',
                    'quantity': random.randint(1, 10),
                    'price': round(random.uniform(10, 1000), 2),
                    'metadata': {f'key_{j}': generate_unique_id() for j in range(10)}
                }
                for i in range(random.randint(5, 20))
            ],
            'debug_info': {
                'stack_trace': '\n'.join([
                    f'    at function_{i}() in file_{random.randint(1,1000)}.js:{random.randint(1,1000)}'
                    for i in range(random.randint(10, 30))
                ]),
                'memory_usage': {f'pool_{i}': random.randint(1000000, 100000000) for i in range(10)},
                'timing': {f'phase_{i}': round(random.uniform(0.001, 2.0), 3) for i in range(20)}
            }
        })

def generate_heavy_log_line():
    timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]
    level = random.choice(['ERROR', 'WARN', 'INFO', 'DEBUG', 'TRACE'])
    
    # Mix of different log types with more unique content
    log_type = random.choice(['json', 'stacktrace', 'metrics', 'event', 'sql'])
    
    if log_type == 'json':
        size = random.choice(['small', 'medium', 'large'])
        return f'[{timestamp}] {level}: JSON Response - {generate_json_payload(size)}'
    
    elif log_type == 'stacktrace':
        error_id = generate_unique_id()
        stack_lines = [f'    at {generate_unique_id()}.method() in module_{random.randint(1,1000)}.py:{random.randint(1,1000)}' 
                      for _ in range(random.randint(5, 20))]
        return f'[{timestamp}] {level}: Error {error_id}\n' + '\n'.join(stack_lines)
    
    elif log_type == 'metrics':
        metrics = {
            'request_id': generate_unique_id(),
            'duration_ms': random.randint(1, 5000),
            'memory_mb': random.randint(100, 1000),
            'cpu_percent': random.uniform(0, 100),
            'gc_runs': random.randint(0, 100),
            'cache_hits': random.randint(0, 10000),
            'cache_misses': random.randint(0, 1000),
            'db_queries': random.randint(1, 100),
            'external_api_calls': random.randint(0, 20)
        }
        return f'[{timestamp}] {level}: Metrics - {json.dumps(metrics)}'
    
    elif log_type == 'event':
        event = {
            'event_id': generate_unique_id(),
            'type': random.choice(['USER_LOGIN', 'PURCHASE', 'API_CALL', 'ERROR', 'SYSTEM_EVENT']),
            'user_id': f'user_{random.randint(1, 1000000)}',
            'session_id': generate_unique_id(),
            'metadata': {f'field_{i}': generate_unique_id() for i in range(random.randint(5, 15))}
        }
        return f'[{timestamp}] {level}: Event - {json.dumps(event)}'
    
    else:  # sql
        table = random.choice(['users', 'orders', 'products', 'sessions', 'analytics'])
        query_id = generate_unique_id()
        return (f'[{timestamp}] {level}: SQL Query {query_id} - '
                f'SELECT * FROM {table} WHERE id IN ({",".join([str(random.randint(1, 1000000)) for _ in range(random.randint(10, 50))])}) '
                f'AND status = "active" AND created_at > "2024-01-01" ORDER BY updated_at DESC LIMIT 1000')

def write_heavy_logs_continuously(file_path, rate_per_second, burst_mode=False):
    """Write heavy logs to stress test memory."""
    print(f"Writing heavy logs to {file_path} at {rate_per_second} lines/sec")
    
    with open(file_path, 'a', buffering=8192) as f:  # Larger buffer
        while True:
            if burst_mode:
                # Burst mode: write many lines at once
                burst_size = random.randint(100, 500)
                lines = []
                for _ in range(burst_size):
                    lines.append(generate_heavy_log_line() + '\n')
                f.writelines(lines)
                f.flush()
                time.sleep(random.uniform(0.5, 2.0))
            else:
                # Steady mode
                lines_per_batch = min(50, rate_per_second)
                lines = []
                for _ in range(lines_per_batch):
                    lines.append(generate_heavy_log_line() + '\n')
                f.writelines(lines)
                f.flush()
                time.sleep(lines_per_batch / rate_per_second)

def main():
    parser = argparse.ArgumentParser(description='Generate heavy logs to stress test memory usage')
    parser.add_argument('files', nargs='+', help='Log files to generate')
    parser.add_argument('--rate', type=int, default=100, help='Lines per second per file')
    parser.add_argument('--burst', action='store_true', help='Use burst mode')
    parser.add_argument('--clear', action='store_true', help='Clear files before starting')
    
    args = parser.parse_args()
    
    if args.clear:
        for file_path in args.files:
            with open(file_path, 'w') as f:
                f.write('')
    
    threads = []
    for file_path in args.files:
        thread = threading.Thread(
            target=write_heavy_logs_continuously,
            args=(file_path, args.rate, args.burst),
            daemon=True
        )
        thread.start()
        threads.append(thread)
    
    print(f"\nGenerating heavy logs for {len(args.files)} files...")
    print("These logs are designed to stress test memory with:")
    print("  - High cardinality data (many unique strings)")
    print("  - Large JSON payloads")
    print("  - Unique IDs that defeat string deduplication")
    print("\nPress Ctrl+C to stop\n")
    
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\nStopping log generation...")

if __name__ == '__main__':
    main()