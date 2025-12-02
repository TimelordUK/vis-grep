#!/usr/bin/env python3
import argparse
import time
import random
import os
from datetime import datetime
import threading

# Log templates with various sizes
LOG_TEMPLATES = {
    'short': [
        '[{timestamp}] INFO: Request processed successfully',
        '[{timestamp}] DEBUG: Cache hit for key: {key}',
        '[{timestamp}] WARN: Slow query detected: {ms}ms',
        '[{timestamp}] ERROR: Connection timeout to {host}',
    ],
    'medium': [
        '[{timestamp}] INFO: Processing batch job {job_id} with {count} items, estimated time: {time}s',
        '[{timestamp}] DEBUG: SQL Query: SELECT * FROM users WHERE id = {user_id} AND status = "active" ORDER BY created_at DESC LIMIT {limit}',
        '[{timestamp}] WARN: Memory usage at {percent}%, consider increasing heap size or optimizing queries',
        '[{timestamp}] ERROR: Failed to process payment {payment_id}: Invalid card number {card} for user {user_id}',
    ],
    'long': [
        '[{timestamp}] INFO: Detailed system status - CPU: {cpu}%, Memory: {memory}MB/{total_memory}MB, Disk: {disk}%, Active connections: {connections}, Queue depth: {queue}, Processing rate: {rate}/s, Error rate: {errors}%, Uptime: {uptime}h',
        '[{timestamp}] DEBUG: Full request trace - Method: {method}, Path: {path}, Headers: {{User-Agent: {agent}, Accept: {accept}, Content-Type: {content_type}}}, Body: {body}, Response time: {response_time}ms, Status: {status}',
        '[{timestamp}] ERROR: Stack trace:\n    at processRequest (app.js:{line1}:{col1})\n    at handleConnection (server.js:{line2}:{col2})\n    at Socket.emit (events.js:{line3}:{col3})\n    at TCP.onread (net.js:{line4}:{col4})\n  Original error: {error_msg}',
    ],
    'huge': [
        '[{timestamp}] DEBUG: ' + 'X' * 500 + ' - Large payload detected',
        '[{timestamp}] INFO: Processing large dataset with ' + str(random.randint(100, 1000)) + ' records',
    ]
}

def generate_timestamp():
    return datetime.now().strftime('%Y-%m-%d %H:%M:%S.%f')[:-3]

def generate_log_line(template_type='mixed'):
    if template_type == 'mixed':
        # Mix of different sizes with weights
        weights = [0.6, 0.25, 0.10, 0.05]  # short, medium, long, huge
        template_type = random.choices(['short', 'medium', 'long', 'huge'], weights=weights)[0]
    
    template = random.choice(LOG_TEMPLATES[template_type])
    
    # Generate random values for placeholders
    values = {
        'timestamp': generate_timestamp(),
        'key': f'cache_key_{random.randint(1000, 9999)}',
        'ms': random.randint(100, 5000),
        'host': f'server{random.randint(1, 10)}.example.com',
        'job_id': f'JOB-{random.randint(10000, 99999)}',
        'count': random.randint(10, 1000),
        'time': random.randint(1, 300),
        'user_id': random.randint(1000, 99999),
        'limit': random.randint(10, 100),
        'percent': random.randint(60, 95),
        'payment_id': f'PAY-{random.randint(100000, 999999)}',
        'card': f'****{random.randint(1000, 9999)}',
        'cpu': random.randint(10, 90),
        'memory': random.randint(1000, 8000),
        'total_memory': 8192,
        'disk': random.randint(20, 80),
        'connections': random.randint(10, 1000),
        'queue': random.randint(0, 500),
        'rate': random.randint(100, 10000),
        'errors': random.uniform(0.01, 5.0),
        'uptime': random.randint(1, 720),
        'method': random.choice(['GET', 'POST', 'PUT', 'DELETE']),
        'path': f'/api/v1/{random.choice(["users", "products", "orders"])}/{random.randint(1, 1000)}',
        'agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
        'accept': 'application/json',
        'content_type': 'application/json',
        'body': '{"field": "value"}',
        'response_time': random.randint(10, 1000),
        'status': random.choice([200, 201, 400, 401, 404, 500]),
        'line1': random.randint(100, 500),
        'col1': random.randint(1, 80),
        'line2': random.randint(100, 500),
        'col2': random.randint(1, 80),
        'line3': random.randint(100, 500),
        'col3': random.randint(1, 80),
        'line4': random.randint(100, 500),
        'col4': random.randint(1, 80),
        'error_msg': random.choice(['Connection refused', 'Timeout exceeded', 'Invalid token', 'Database error']),
    }
    
    return template.format(**values)

def write_logs_continuously(file_path, rate_per_second, burst_mode=False, template_type='mixed'):
    """Write logs to a file continuously at the specified rate."""
    print(f"Writing to {file_path} at {rate_per_second} lines/sec (burst_mode={burst_mode})")
    
    with open(file_path, 'a') as f:
        while True:
            if burst_mode:
                # Burst mode: write many lines at once, then pause
                burst_size = random.randint(50, 200)
                for _ in range(burst_size):
                    f.write(generate_log_line(template_type) + '\n')
                f.flush()
                time.sleep(random.uniform(0.5, 2.0))
            else:
                # Steady mode: write at consistent rate
                lines_per_batch = min(10, rate_per_second)
                for _ in range(lines_per_batch):
                    f.write(generate_log_line(template_type) + '\n')
                f.flush()
                time.sleep(lines_per_batch / rate_per_second)

def main():
    parser = argparse.ArgumentParser(description='Generate log files with configurable rate and content')
    parser.add_argument('files', nargs='+', help='Log files to generate')
    parser.add_argument('--rate', type=int, default=100, help='Lines per second per file (default: 100)')
    parser.add_argument('--burst', action='store_true', help='Use burst mode (write in bursts)')
    parser.add_argument('--template', choices=['short', 'medium', 'long', 'huge', 'mixed'], 
                        default='mixed', help='Log template type (default: mixed)')
    parser.add_argument('--clear', action='store_true', help='Clear files before starting')
    
    args = parser.parse_args()
    
    # Clear files if requested
    if args.clear:
        for file_path in args.files:
            with open(file_path, 'w') as f:
                f.write('')
            print(f"Cleared {file_path}")
    
    # Start a thread for each file
    threads = []
    for file_path in args.files:
        thread = threading.Thread(
            target=write_logs_continuously,
            args=(file_path, args.rate, args.burst, args.template),
            daemon=True
        )
        thread.start()
        threads.append(thread)
    
    print(f"\nGenerating logs for {len(args.files)} files...")
    print("Press Ctrl+C to stop\n")
    
    try:
        # Keep the main thread alive
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\nStopping log generation...")

if __name__ == '__main__':
    main()