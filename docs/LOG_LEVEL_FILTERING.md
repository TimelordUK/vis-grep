# Log Level Filtering Design

## Overview

Add the ability to filter log output by severity level in tail mode, making it easier to focus on important messages (errors, warnings) while hiding verbose output (debug, trace).

## Core Concepts

### Log Level Hierarchy

```
FATAL/CRITICAL  (Highest severity)
ERROR
WARN
INFO
DEBUG
TRACE           (Lowest severity)
UNKNOWN         (No detectable level)
```

### Filter Modes

1. **ALL** - Show everything (default)
2. **INFO+** - Show INFO, WARN, ERROR, FATAL (hide DEBUG, TRACE)
3. **WARN+** - Show WARN, ERROR, FATAL (hide INFO, DEBUG, TRACE)
4. **ERROR** - Show only ERROR and FATAL

## Where Filtering Applies

### 1. Combined Output (Main Filter)
- **Primary filtering location** - most important use case
- Filter applied to aggregated output from all files
- Shows filtered view across entire log stream
- Lines without detectable log level (UNKNOWN) are shown by default
  - Can be toggled via "Show UNKNOWN" checkbox

### 2. File List (Tree View)
- **Activity indicators updated** to show level-specific counts
- Current: `app.log (+17 lines)`
- Enhanced: `app.log (+5 ERR, +12 WARN)`
- Files with errors could have red indicator dot
- Files with warnings could have yellow indicator dot

### 3. Preview Pane
- **Shows ALL lines by default** (unfiltered)
- Rationale: Preview is for detailed inspection
- Optional: Add "Apply filter to preview" toggle checkbox
- The `/` search filter remains independent of level filtering

## UI Design

### Combined Output Panel Header

```
┌────────────────────────────────────────────────────────┐
│ Combined Output                                        │
├────────────────────────────────────────────────────────┤
│ Filter Level: [ALL ▼] [INFO+] [WARN+] [ERROR]         │
│ ☐ Show UNKNOWN lines  ☐ Apply to preview              │
├────────────────────────────────────────────────────────┤
│ [ERROR] app.log: Failed to connect to database        │
│ [WARN]  app.log: Retrying connection (attempt 2/3)    │
│ [ERROR] db.log:  Connection timeout after 30s         │
│ [FATAL] app.log: Unable to start - exiting            │
└────────────────────────────────────────────────────────┘
```

### Enhanced File List Indicators

```
Tree View:
├─ 📱 Application Logs
│  ├─ ● app.log        (+5 ERR, +12 WARN)  🔴  <- Red dot for errors
│  ├─ ○ access.log     (idle)
│  └─ ● debug.log      (+150 DBG)          ○   <- No errors/warns
└─ 🗄️ Database Logs
   └─ ● db.log         (+2 ERR)            🔴
```

## Implementation Details

### State Management

Add to `TailState`:
```rust
pub struct LogLevelFilter {
    pub active: bool,
    pub minimum_level: LogLevel,      // INFO, WARN, ERROR, etc.
    pub show_unknown: bool,            // Show lines without detectable level
    pub apply_to_preview: bool,        // Apply filter to preview pane

    // Statistics (updated as lines are processed)
    pub level_counts: HashMap<LogLevel, usize>,
}
```

### Filtering Logic

```rust
impl LogLevelFilter {
    fn should_show_line(&self, line: &str, detector: &LogLevelDetector) -> bool {
        if !self.active {
            return true;  // Filter disabled, show everything
        }

        let detected_level = detector.detect(line);

        match detected_level {
            LogLevel::Unknown => self.show_unknown,
            _ => detected_level.severity() >= self.minimum_level.severity()
        }
    }
}
```

### Per-File Level Statistics

Track counts per file for enhanced activity indicators:

```rust
pub struct TailedFile {
    // ... existing fields ...

    // Level-specific line counts (since last read)
    pub error_count: usize,
    pub warn_count: usize,
    pub info_count: usize,
    pub debug_count: usize,
}
```

## User Interactions

### Keyboard Shortcuts

- `L` - Cycle through filter levels (ALL → INFO+ → WARN+ → ERROR → ALL)
- `Shift+L` - Cycle backwards
- `U` - Toggle "Show UNKNOWN" lines
- `/` - Continue to work for text-based filtering (independent)

### Mouse Interactions

- Click filter level buttons to switch modes
- Hover over file in tree to see detailed level breakdown
- Click checkboxes to toggle options

## Edge Cases & Considerations

### 1. Lines Without Log Levels
**Problem:** Not all lines have detectable log levels (stack traces, continuation lines, etc.)

**Solution:**
- Treat as UNKNOWN level
- Show by default (checkbox: "Show UNKNOWN lines")
- Could implement "sticky level" - inherit level from previous line in same file

### 2. Multi-line Log Entries
**Problem:** Stack traces, JSON logs span multiple lines

**Solution (Future Enhancement):**
- Detect and group related lines
- Apply parent line's level to children
- For now: treat each line independently

### 3. Performance
**Problem:** Detecting log level for every line could impact performance

**Solution:**
- Cache detector patterns (already using `Lazy` static)
- Run detection once when line is read
- Store detected level with line data

### 4. Custom Log Formats
**Problem:** Some logs use non-standard formats

**Solution:**
- Already support custom patterns in config.yaml
- User can add patterns like: `[("MY_CUSTOM_ERROR", "ERROR")]`
- Provide good defaults that cover 90% of cases

### 5. Filter Persistence
**Question:** Should filter state persist between sessions?

**Decision:**
- Start with in-memory state (resets on restart)
- Future: Add to saved layout YAML if users request it

## Testing Strategy

### Test Data

Generate test logs with:
```bash
./generate_test_logs.sh --levels all --count 1000
```

Should produce mix of:
- Lines with clear levels: `[ERROR]`, `[INFO]`, etc.
- Lines without levels: stack traces, JSON, plain text
- Multi-line entries
- Different format styles

### Test Cases

1. **Basic Filtering**
   - Set to ERROR - verify only errors shown
   - Set to WARN+ - verify warn and error shown
   - Toggle UNKNOWN - verify behavior changes

2. **Combined Output**
   - Multiple files with different log levels
   - Verify correct interleaving and filtering
   - Check that timestamps remain in order

3. **File Statistics**
   - Generate errors in one file
   - Verify tree view shows error count
   - Verify indicator color changes

4. **Performance**
   - 1000 lines/second across 10 files
   - Filter should not cause lag
   - Memory usage should remain stable

## Future Enhancements

### Phase 2 Features (Not in Initial Implementation)

1. **Custom Filter Expressions**
   ```yaml
   filters:
     - name: "Errors except heartbeat"
       expression: "ERROR AND NOT heartbeat"
   ```

2. **Level-based Highlighting**
   - Different background colors for different levels
   - Already have text color; could add subtle bg

3. **Filter Presets**
   - Save common filter combinations
   - Quick switch between "Production" vs "Debug" views

4. **Time-based Filtering**
   - "Show only errors from last 5 minutes"
   - Combine with level filtering

5. **Statistical Dashboard**
   - Show errors/min rate
   - Alert when error rate spikes
   - Graph of levels over time

## Implementation Checklist

- [ ] Add `LogLevelFilter` struct to filter module
- [ ] Add filter state to `TailState`
- [ ] Implement filtering logic for combined output
- [ ] Add UI controls (buttons, checkboxes) to combined output header
- [ ] Update file list to show level-specific counts
- [ ] Add colored indicators for files with errors/warnings
- [ ] Implement keyboard shortcuts (`L`, `U`)
- [ ] Update line processing to track level counts per file
- [ ] Add "Apply to preview" option
- [ ] Test with various log formats
- [ ] Update documentation with examples
- [ ] Create test data generator

## Config File Example

```yaml
log_format:
  color_preset: Monochrome

  # Custom patterns for your specific log format
  custom_patterns:
    - ["\\[SEVERE\\]", "ERROR"]
    - ["\\[NOTICE\\]", "INFO"]

  # Default filter settings (optional)
  default_filter:
    minimum_level: INFO      # Hide DEBUG and TRACE by default
    show_unknown: true       # Show lines without detectable level
    apply_to_preview: false  # Preview always shows all
```

## Open Questions

1. Should filter affect the activity indicator (●/○)?
   - Proposal: No - indicator shows any activity regardless of filter
   - Count display respects filter

2. Should we add a "sticky level" feature for multi-line entries?
   - Proposal: Not in v1 - adds complexity
   - Revisit based on user feedback

3. Should combined output show source file for each line?
   - Current: Shows file name with each line
   - Keep as-is for now

4. Keyboard shortcut conflicts?
   - `L` is available (not used currently)
   - `U` is available
   - Verify no conflicts

## Success Criteria

Implementation is successful when:
1. User can quickly filter to ERROR-only view with 1 click
2. File tree shows which files have errors at a glance
3. Filtering works smoothly with 1000+ lines/second
4. Preview pane remains unaffected (shows all detail)
5. Filter state is intuitive and doesn't require documentation
6. Works with variety of log formats (tested with 5+ different styles)
