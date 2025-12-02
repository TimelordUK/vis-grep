# Memory Optimization Guide

## Problem: High Memory Usage in Tail Mode

When tailing busy log files, memory usage can grow from 400MB to 1GB+ in seconds.

## Root Cause

The issue is **not** in the file monitoring logic, but in the GUI rendering:

1. **No UI virtualization**: egui renders ALL lines in the buffer (up to 10,000), even though only ~50 are visible
2. Each UI element requires ~200+ bytes for widget state, layout, and interaction tracking  
3. With 10,000 lines, this creates 100s of MB of UI overhead

## Immediate Workaround

Reduce buffer sizes in `~/.config/vis-grep/config.yaml`:

```yaml
ui:
  max_buffer_lines: 1000      # Default: 10000
  preview_follow_lines: 1000  # Default: varies
```

This reduces memory usage by ~90% while maintaining reasonable functionality.

## Testing

The `test_memory/` directory contains tools to analyze memory usage:

1. **Headless test** - Shows core logic uses only ~10-20MB
2. **Log generators** - Create realistic high-volume test data
3. **Memory profiler** - Tracks memory growth over time

To run tests:
```bash
cd test_memory
./build_headless.sh
./demo_memory_issue.sh
```

## Long-term Solution

Implement UI virtualization to only render visible lines:

```rust
// Instead of rendering all lines:
for log_line in &buffer {  // 10,000 iterations!
    ui.label(&log_line.content);
}

// Only render visible lines:
let visible_lines = ui.available_height() / line_height;
for log_line in buffer.iter().skip(scroll_pos).take(visible_lines) {
    ui.label(&log_line.content);
}
```

This would reduce memory usage to negligible levels regardless of buffer size.