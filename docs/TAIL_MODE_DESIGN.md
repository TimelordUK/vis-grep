# Tail/Follow Mode Design Document

## Problem Statement

Users need to monitor multiple live log files simultaneously, similar to `tail -f` but with:
- Multiple files at once (up to 20+)
- Visual indication of which files are active
- Built-in filtering/search on live content
- Reliable detection on network shares (Windows)
- Better UX than command-line tail

### Real-World Use Cases

1. **Multiple FIX Logs**: Monitor 20 different FIX session logs, see which sessions are active
2. **Network Shares**: Files on Windows network drives where file modification time is unreliable
3. **Filter While Tailing**: Only show lines matching certain patterns (e.g., errors, specific order IDs)
4. **Activity Indicators**: Visual feedback showing which files have new activity

### Challenges

#### Windows Network Share Issues
- File modification time (`mtime`) can be cached/unreliable on SMB/CIFS shares
- `tail -f` using inotify doesn't work well on network drives
- Need alternative detection methods:
  - File size changes (more reliable)
  - Periodic polling with size comparison
  - Hash of last N bytes

#### UI/UX Complexity
- Don't want to clutter the grep interface
- Need clear separation between "grep mode" and "tail mode"
- Multiple files showing updates simultaneously = complex UI

## Design Options

### Option 1: Separate Tab/Mode Toggle

```
┌─────────────────────────────────────────────────────┐
│ [ Grep Mode ]  [ Tail Mode ]                        │
└─────────────────────────────────────────────────────┘

When in Tail Mode:
┌─────────────────────────────────────────────────────┐
│ Files Being Tailed:                                 │
│                                                      │
│ [ + Add File ] [ + Add Folder Pattern ]             │
│                                                      │
│ ┌─────────────────────────────────────────────────┐ │
│ │ ● fix_session_1.log          (12 new lines)    │ │
│ │ ○ fix_session_2.log          (idle)            │ │
│ │ ● fix_session_3.log          (5 new lines)     │ │
│ │ ○ error.log                  (idle)            │ │
│ └─────────────────────────────────────────────────┘ │
│                                                      │
│ Filter: [___________________]  [●] Regex  [ ] Case  │
│                                                      │
│ ┌─────────────────────────────────────────────────┐ │
│ │ [fix_session_1.log] 14:32:01 - New order recv  │ │
│ │ [fix_session_3.log] 14:32:02 - Fill received   │ │
│ │ [fix_session_1.log] 14:32:03 - Order ack       │ │
│ │ ...                                             │ │
│ └─────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

**Pros:**
- Clean separation from grep mode
- Can have different UI optimized for tailing
- No confusion about which mode you're in

**Cons:**
- More UI complexity
- Need state management for two different modes

### Option 2: "Follow" Toggle in Existing UI

Add a "Follow" checkbox to current search results:

```
┌─────────────────────────────────────────────────────┐
│ Search Results:  [●] Follow Updates                 │
│                                                      │
│ src/main.rs (5 matches) [Activity: ●]               │
│   Line 42: fn main() {                              │
│   Line 156: let result = search();                  │
│                                                      │
│ src/config.rs (2 matches) [Activity: ○]             │
│   Line 23: pub struct Config {                      │
└─────────────────────────────────────────────────────┘
```

**Pros:**
- Simpler implementation
- Leverages existing search/filter infrastructure
- Gradual feature addition

**Cons:**
- Mixing grep and tail concepts might be confusing
- Existing UI not optimized for live updates

### Option 3: Separate "Tail" Application/Window

Launch a separate window from VisGrep for tailing:

**Pros:**
- Complete separation
- Could even be a separate binary
- No contamination of grep UI

**Cons:**
- More work to implement
- Separate application to maintain

## Recommended Approach: Option 1 (Tab/Mode Toggle)

Start with two distinct modes in the same application:

### Phase 1: Basic Tail Mode
1. Mode toggle at top: `[ Grep ]  [ Tail ]`
2. In Tail mode:
   - List of files to tail (add/remove)
   - Combined output stream
   - Auto-scroll to bottom
   - Color-code by filename
   - Activity indicators (● active, ○ idle)

### Phase 2: File Watching
1. Reliable file change detection:
   ```rust
   struct TailedFile {
       path: PathBuf,
       last_size: u64,
       last_position: u64,
       active: bool,
       last_activity: SystemTime,
   }
   ```

2. Detection strategy (especially for Windows network shares):
   ```rust
   fn check_for_updates(file: &mut TailedFile) -> Result<Vec<String>> {
       let current_size = fs::metadata(&file.path)?.len();

       if current_size > file.last_size {
           // File grew - read new content
           let new_lines = read_from_position(&file.path, file.last_position)?;
           file.last_size = current_size;
           file.last_position = current_size;
           file.active = true;
           file.last_activity = SystemTime::now();
           Ok(new_lines)
       } else if current_size < file.last_size {
           // File was truncated/rotated - restart from beginning
           file.last_position = 0;
           file.last_size = current_size;
           Ok(vec!["[FILE TRUNCATED/ROTATED]".to_string()])
       } else {
           // No change
           file.active = false;
           Ok(vec![])
       }
   }
   ```

3. Polling interval: 250ms (configurable)

### Phase 3: Filtering
- Apply regex filter to live output
- Only show matching lines
- Highlight matches (like matched line panel)
- Option to show context lines around matches

### Phase 4: Advanced Features
- Pause/resume following
- Clear output
- Export visible lines
- Bookmarks in output
- Jump to file in grep mode (if pattern found)

## Technical Implementation

### File Watching Strategy

**For Reliable Network Share Detection:**

```rust
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

struct FileWatcher {
    file: File,
    last_size: u64,
    poll_interval_ms: u64,
}

impl FileWatcher {
    fn check_updates(&mut self) -> Result<Vec<String>, std::io::Error> {
        let metadata = self.file.metadata()?;
        let current_size = metadata.len();

        if current_size > self.last_size {
            // New data available
            self.file.seek(SeekFrom::Start(self.last_size))?;
            let reader = BufReader::new(&self.file);
            let new_lines: Vec<String> = reader.lines()
                .filter_map(|l| l.ok())
                .collect();

            self.last_size = current_size;
            Ok(new_lines)
        } else if current_size < self.last_size {
            // File truncated - restart
            self.last_size = 0;
            self.file.seek(SeekFrom::Start(0))?;
            Ok(vec!["[FILE ROTATED]".to_string()])
        } else {
            Ok(vec![])
        }
    }
}
```

**Why This Works Better Than mtime:**
- File size is more reliably updated on network shares
- Keep file handle open (stays positioned)
- Direct size comparison is fast
- Works on Windows SMB shares

### UI State Management

```rust
enum AppMode {
    Grep,
    Tail,
}

struct TailState {
    files: Vec<TailedFile>,
    output_buffer: VecDeque<LogLine>,  // Ring buffer
    filter_pattern: String,
    paused: bool,
    auto_scroll: bool,
    max_lines: usize,  // e.g., 10000
}

struct LogLine {
    timestamp: SystemTime,
    filename: String,
    line_number: usize,
    content: String,
    matches_filter: bool,
}
```

### Update Loop

```rust
// In main event loop
if matches!(self.mode, AppMode::Tail) && !self.tail_state.paused {
    // Poll files every 250ms
    if self.last_tail_update.elapsed() > Duration::from_millis(250) {
        for tailed_file in &mut self.tail_state.files {
            if let Ok(new_lines) = tailed_file.watcher.check_updates() {
                for line in new_lines {
                    let log_line = LogLine {
                        timestamp: SystemTime::now(),
                        filename: tailed_file.display_name.clone(),
                        line_number: tailed_file.line_count,
                        content: line,
                        matches_filter: self.tail_state.matches_filter(&line),
                    };

                    self.tail_state.output_buffer.push_back(log_line);
                    tailed_file.line_count += 1;
                }

                // Trim buffer if too large
                while self.tail_state.output_buffer.len() > self.tail_state.max_lines {
                    self.tail_state.output_buffer.pop_front();
                }
            }
        }
        self.last_tail_update = Instant::now();
    }
}
```

## UI Mockup (Tail Mode)

```
┌────────────────────────────────────────────────────────────────┐
│ VisGrep                                                         │
│ ┌──────────┬──────────┐                                        │
│ │   Grep   │   Tail   │  ← Mode tabs                           │
│ └──────────┴──────────┘                                        │
│                                                                 │
│ Tailed Files:                        [+ Add File] [+ Pattern]  │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ ● fix_session_1.log        Last: 14:32:03  (12 new) [×]   │ │
│ │ ○ fix_session_2.log        Last: 14:29:15  (idle)   [×]   │ │
│ │ ● error.log                Last: 14:32:01  (3 new)  [×]   │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                 │
│ Filter: [35=8.*150=8___________] [●] Regex  [Clear]           │
│                                                                 │
│ Output:                    [ Pause ]  [ Clear ]  [Export]      │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │14:32:01 [error.log] ERROR: Connection timeout to server   │ │
│ │14:32:02 [fix_session_1.log] 8=FIX.4.2|35=8|150=8|39=8...  │ │
│ │14:32:03 [fix_session_1.log] 8=FIX.4.2|35=D|11=ORD001...   │ │
│ │14:32:03 [error.log] ERROR: Failed to parse message        │ │
│ │...                                                         │ │
│ │                                                             │ │
│ │ [Auto-scroll ✓]                    Showing 2847 lines     │ │
│ └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Configuration

Add to config.yaml:

```yaml
tail_mode:
  poll_interval_ms: 250
  max_output_lines: 10000
  color_by_file: true
  show_timestamps: true
  default_files:
    - path: "~/logs/fix_session_1.log"
      name: "FIX Session 1"
    - path: "~/logs/error.log"
      name: "Errors"
```

## Alternative Detection Methods (If Size-Based Fails)

### Method 1: Hash Last Block
```rust
fn file_changed(path: &Path, last_hash: &mut u64) -> bool {
    if let Ok(file) = File::open(path) {
        let size = file.metadata().ok()?.len();
        if size > 1024 {
            // Hash last 1KB
            let mut buffer = [0u8; 1024];
            file.seek(SeekFrom::End(-1024)).ok()?;
            file.read_exact(&mut buffer).ok()?;

            let hash = calculate_hash(&buffer);
            if hash != *last_hash {
                *last_hash = hash;
                return true;
            }
        }
    }
    false
}
```

### Method 2: notify crate (with caveats)
```rust
// Works on local drives, unreliable on network shares
use notify::{Watcher, RecursiveMode, watcher};

// Only use as fallback for local files
```

## Performance Considerations

- **Memory**: Ring buffer prevents unbounded growth
- **CPU**: Polling 20 files @ 250ms = 80 checks/sec (negligible)
- **I/O**: Only read new bytes, not entire file
- **Network**: Minimal - just size check + new data read

## Dependencies Needed

```toml
[dependencies]
# Possibly:
notify = "6.0"  # Optional, for local file watching
```

Most of the implementation can use `std::fs` for reliability.

## Comparison with Existing Tools

### BareTail
- ✅ Multiple files
- ✅ Highlighting
- ✅ Network share support
- ❌ No vim keybindings
- ❌ No integrated grep

### less +F
- ✅ Single file follow
- ❌ Doesn't work well on Windows
- ❌ No multiple files
- ❌ Poor network share support

### VisGrep Tail Mode (Proposed)
- ✅ Multiple files
- ✅ Network share support (size-based detection)
- ✅ Vim keybindings (in grep mode)
- ✅ Integrated grep + tail
- ✅ Regex filtering on live output
- ✅ Cross-platform

## Open Questions

1. **Mode Switching**: Can you switch from Grep → Tail with current results?
   - Maybe: "Tail these files" button in grep results?

2. **File Selection**:
   - Manual add/remove?
   - Glob patterns (e.g., `logs/*.log`)?
   - From search results?

3. **Output Format**:
   - Interleaved (all files mixed)?
   - Split panels (one per file)?
   - Tabbed (switch between files)?

4. **Color Coding**:
   - Different color per file?
   - Consistent with search highlighting?

5. **File Rotation**:
   - Detect when file is renamed/deleted?
   - Follow rotated files (file.log → file.log.1)?

## Implementation Priority

**High Priority (Phase 1):**
- Mode toggle
- Basic file watching (size-based)
- Simple output display
- Pause/resume

**Medium Priority (Phase 2):**
- Filtering
- Multiple files with activity indicators
- Config persistence

**Low Priority (Phase 3):**
- Advanced rotation detection
- Split panel view
- Export functionality

## Next Steps

1. ✅ Document design (this file)
2. Get user feedback on approach
3. Prototype basic file watching logic
4. Build UI for mode toggle
5. Implement Phase 1 features

## Notes

This is a significant feature that would make VisGrep a complete log analysis tool. The combination of powerful grep + live tail in one tool with vim keybindings would be very unique.

Key insight: **Use file size changes instead of mtime for network share reliability.**
