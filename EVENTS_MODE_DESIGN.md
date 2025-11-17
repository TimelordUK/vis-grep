# Events Mode Design

## Overview
Extract interesting multi-line log events (e.g., JSON payloads, stack traces, API requests) from log files using regex patterns, displaying them in a focused view with navigation to source.

## Architecture: File Watch Manager (Foundation)

### Problem
Currently, file watching is embedded in `TailedFile`. Multiple modes (Tail, Less, Events) will need file updates, leading to:
- Duplicated polling logic
- Multiple reads of the same file
- Inconsistent state tracking

### Solution: Centralized FileWatchManager

```rust
/// Centralized file watching and update management
struct FileWatchManager {
    /// Watched files with their state
    files: HashMap<PathBuf, WatchedFile>,

    /// Subscribers per file
    subscribers: HashMap<PathBuf, Vec<FileSubscriber>>,
}

struct WatchedFile {
    path: PathBuf,
    display_name: String,

    // File state
    last_position: u64,
    last_size: u64,
    last_modified: SystemTime,

    // Error handling
    consecutive_errors: u32,
    last_error_time: Option<Instant>,

    // Cached metadata
    total_lines: usize,
    total_bytes: u64,
}

enum FileSubscriber {
    Tail { mode_id: usize },
    Less { mode_id: usize },
    Events { mode_id: usize },
}

enum FileUpdate {
    NewLines { lines: Vec<String>, start_line: usize },
    Truncated,
    Deleted,
    Error(String),
}
```

### Benefits
- ✅ Single source of truth for file state
- ✅ Efficient: read each file once, notify all subscribers
- ✅ Consistent error handling and recovery
- ✅ Easy to add new modes (just subscribe)

## Events Mode Design

### YAML Configuration
```yaml
# In existing tail layout or separate events.yaml
events:
  - name: "API Requests"
    # Regex to detect start of event
    start_pattern: '^\[\d{4}-\d{2}-\d{2}.*\] REQUEST:'
    # Regex to detect end (next log line starts)
    end_pattern: '^\[\d{4}-\d{2}-\d{2}'
    # Max lines to capture for one event
    max_lines: 50
    # Color/tag for this event type
    color: "#4CAF50"

  - name: "Exceptions"
    start_pattern: '^Exception|^Error:|^Traceback'
    end_pattern: '^\[\d{4}-\d{2}-\d{2}'
    max_lines: 100
    color: "#F44336"

  - name: "JSON Payloads"
    start_pattern: '^\[\d{4}-\d{2}-\d{2}.*\] PAYLOAD: \{'
    end_pattern: '^\[\d{4}-\d{2}-\d{2}'
    max_lines: 200
    color: "#2196F3"
```

### Data Structure
```rust
struct EventsState {
    /// Active event patterns
    patterns: Vec<EventPattern>,

    /// Captured events (ringbuffer to limit memory)
    events: VecDeque<CapturedEvent>,
    max_events: usize, // e.g., 1000

    /// Selected event for preview
    selected_event: Option<usize>,

    /// Preview buffer (reuse BufferWindow!)
    preview_buffer: BufferWindow,
}

struct EventPattern {
    name: String,
    start_regex: Regex,
    end_regex: Regex,
    max_lines: usize,
    color: Color32,
}

struct CapturedEvent {
    /// Which pattern matched
    pattern_name: String,

    /// Source file
    file_path: PathBuf,

    /// Position in file
    start_line: usize,  // Absolute line in file
    end_line: usize,

    /// The actual content
    lines: Vec<String>,

    /// When captured
    timestamp: Instant,
}
```

### UI Layout

```
┌─────────────────────────────────────────────────────┐
│ [Grep] [Tail] [Events] [Less]                      │
├─────────────────────────────────────────────────────┤
│ Events (23 captured)                   [Clear All] │
├──────────────────┬──────────────────────────────────┤
│ Event List (30%) │ Preview (70%)                    │
│                  │                                  │
│ ● API Request    │ [2024-11-16 15:30:22] REQUEST:  │
│   main.rs:1523   │ POST /api/users                  │
│   15:30:22       │ {                                │
│                  │   "username": "john",            │
│ ● Exception      │   "email": "john@example.com"    │
│   app.log:8234   │ }                                │
│   15:30:25       │                                  │
│                  │ [📁 Open Location] [📝 Editor]   │
│ ● JSON Payload   │                                  │
│   api.log:4521   │                                  │
│   15:30:30       │                                  │
│                  │                                  │
└──────────────────┴──────────────────────────────────┘
```

### Event Capture Logic

```rust
impl EventsMode {
    fn process_new_lines(&mut self, file: &PathBuf, lines: Vec<String>, start_line: usize) {
        let mut current_event: Option<(EventPattern, Vec<String>, usize)> = None;

        for (offset, line) in lines.iter().enumerate() {
            let line_num = start_line + offset;

            // Check if we're currently capturing an event
            if let Some((pattern, event_lines, event_start)) = &mut current_event {
                event_lines.push(line.clone());

                // Check if event ends
                if pattern.end_regex.is_match(line) || event_lines.len() >= pattern.max_lines {
                    // Save the captured event
                    self.save_event(file, pattern.name.clone(), event_start, line_num, event_lines.clone());
                    current_event = None;
                }
            } else {
                // Check if new event starts
                for pattern in &self.patterns {
                    if pattern.start_regex.is_match(line) {
                        current_event = Some((pattern.clone(), vec![line.clone()], line_num));
                        break;
                    }
                }
            }
        }

        // If event is still open, keep it for next batch
        // (store in pending_events HashMap<PathBuf, PendingEvent>)
    }
}
```

### Integration with Preview

**Reuse existing preview infrastructure!**
- Click event → Load lines into `preview_buffer: BufferWindow`
- Use existing TextViewer widget
- "Open Location" → Switch to Tail mode, navigate to file + line
- "Open in Editor" → Use existing `open_file_in_editor(path, line)`

### Phase 1: MVP Implementation

1. **FileWatchManager** (Foundation)
   - Extract file watching from `TailedFile`
   - Implement subscribe/notify pattern
   - Migrate Tail mode to use it

2. **Events Mode Basic**
   - Add Events tab
   - Load event patterns from YAML
   - Capture single-line events first (simpler regex)
   - Display in list with timestamp

3. **Preview Integration**
   - Click event → show in preview pane
   - Reuse TextViewer widget
   - Navigate to source file

4. **Multi-line Events**
   - Implement start/end pattern matching
   - Handle pending events across updates
   - Limit event size

5. **Polish**
   - Color coding by event type
   - Filter events by pattern
   - Export events to file
   - Clear old events (ringbuffer)

## Benefits of This Approach

### Compared to grep
- **Context preservation**: Full multi-line events, not just matching lines
- **Live updates**: Events appear as they happen
- **Structured**: Events grouped by type/pattern
- **Navigable**: Click to jump to source

### Compared to manual log parsing
- **Automatic**: No need to extract/save portions
- **Fast**: Regex is fast, runs as logs arrive
- **Memory efficient**: Ringbuffer limits storage

## Future Enhancements

### Event Correlation
```yaml
correlations:
  - name: "Request/Response Pair"
    start_event: "API Requests"
    end_event: "API Responses"
    match_field: "request_id"  # Extract from regex group
    timeout_ms: 5000
```

### Event Aggregation
- Count events by type
- Show event rate graph
- Highlight bursts/anomalies

### Event Export
- Save matched events to separate file
- JSON export for further analysis
- Copy event to clipboard

## Implementation Priority

**Session 1: FileWatchManager**
- Extract and centralize file watching
- Implement pub/sub for updates
- Migrate Tail mode

**Session 2: Events Mode MVP**
- Add Events tab
- Single-line event capture
- Basic list view

**Session 3: Preview & Navigation**
- Click event → preview
- Jump to source location
- Editor integration

**Session 4: Multi-line Events**
- Complex pattern matching
- Pending event handling
- Size limits

**Session 5: Polish**
- Colors, filters, export
- Performance optimization
- Documentation

## Open Questions

1. **Event storage**: In-memory ringbuffer or SQLite for persistence?
2. **Pattern reload**: Hot-reload YAML without restart?
3. **Multiple files**: One event list or per-file?
4. **Event search**: Should events be searchable with their own filter?
5. **Performance**: How many events/sec can we capture without lag?

## Success Criteria

- ✅ Capture 1000+ events without memory issues
- ✅ Process 10K lines/sec without dropping events
- ✅ Navigate to source with one click
- ✅ Preview shows full event context
- ✅ Multiple event patterns work simultaneously
- ✅ Clear separation from Tail/Grep modes
