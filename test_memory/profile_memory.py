#!/usr/bin/env python3
import subprocess
import time
import psutil
import matplotlib.pyplot as plt
from datetime import datetime
import sys
import os
import signal

def get_process_memory(pid):
    """Get memory stats for a process."""
    try:
        process = psutil.Process(pid)
        mem_info = process.memory_info()
        return {
            'rss': mem_info.rss / 1024 / 1024,  # MB
            'vms': mem_info.vms / 1024 / 1024,  # MB
            'percent': process.memory_percent()
        }
    except:
        return None

def profile_process(command, duration=60, sample_interval=0.5):
    """Run a command and profile its memory usage."""
    print(f"Starting process: {' '.join(command)}")
    
    # Start the process
    process = subprocess.Popen(command)
    pid = process.pid
    
    # Data collection
    timestamps = []
    rss_values = []
    vms_values = []
    
    start_time = time.time()
    
    try:
        while time.time() - start_time < duration:
            mem_stats = get_process_memory(pid)
            if mem_stats:
                current_time = time.time() - start_time
                timestamps.append(current_time)
                rss_values.append(mem_stats['rss'])
                vms_values.append(mem_stats['vms'])
                
                # Print current stats
                print(f"\r[{current_time:6.1f}s] RSS: {mem_stats['rss']:7.1f}MB, VMS: {mem_stats['vms']:7.1f}MB", end='')
                sys.stdout.flush()
            
            time.sleep(sample_interval)
            
            # Check if process is still running
            if process.poll() is not None:
                print("\nProcess terminated early")
                break
    
    except KeyboardInterrupt:
        print("\nProfiling interrupted")
    
    finally:
        # Terminate the process if still running
        if process.poll() is None:
            process.terminate()
            process.wait()
    
    print(f"\n\nProfiling complete. Collected {len(timestamps)} samples.")
    
    # Analysis
    if rss_values:
        print("\nMemory Statistics:")
        print(f"  Initial RSS: {rss_values[0]:.1f}MB")
        print(f"  Final RSS: {rss_values[-1]:.1f}MB")
        print(f"  Peak RSS: {max(rss_values):.1f}MB")
        print(f"  Growth: {rss_values[-1] - rss_values[0]:.1f}MB")
        print(f"  Growth rate: {(rss_values[-1] - rss_values[0]) / (timestamps[-1] / 60):.1f}MB/min")
    
    return timestamps, rss_values, vms_values

def plot_memory_usage(timestamps, rss_values, vms_values, output_file='memory_profile.png'):
    """Create a graph of memory usage over time."""
    plt.figure(figsize=(12, 6))
    
    plt.plot(timestamps, rss_values, label='RSS (Resident Set Size)', linewidth=2)
    plt.plot(timestamps, vms_values, label='VMS (Virtual Memory Size)', linewidth=1, alpha=0.7)
    
    plt.xlabel('Time (seconds)')
    plt.ylabel('Memory (MB)')
    plt.title('Memory Usage Profile')
    plt.legend()
    plt.grid(True, alpha=0.3)
    
    plt.tight_layout()
    plt.savefig(output_file, dpi=150)
    print(f"\nMemory profile graph saved to: {output_file}")

def main():
    if len(sys.argv) < 2:
        print("Usage: python profile_memory.py <command> [args...] [--duration N]")
        sys.exit(1)
    
    # Parse arguments
    command_args = []
    duration = 60
    
    i = 1
    while i < len(sys.argv):
        if sys.argv[i] == '--duration' and i + 1 < len(sys.argv):
            duration = int(sys.argv[i + 1])
            i += 2
        else:
            command_args.append(sys.argv[i])
            i += 1
    
    # Profile the command
    timestamps, rss_values, vms_values = profile_process(command_args, duration)
    
    if timestamps:
        # Generate timestamp for output files
        timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
        output_file = f'memory_profile_{timestamp}.png'
        plot_memory_usage(timestamps, rss_values, vms_values, output_file)
        
        # Save raw data
        data_file = f'memory_data_{timestamp}.csv'
        with open(data_file, 'w') as f:
            f.write('time_seconds,rss_mb,vms_mb\n')
            for t, r, v in zip(timestamps, rss_values, vms_values):
                f.write(f'{t:.1f},{r:.1f},{v:.1f}\n')
        print(f"Raw data saved to: {data_file}")

if __name__ == '__main__':
    main()