# VisGrep Feature Ideas and Roadmap

This document captures feature ideas and implementation concepts discussed during development.

## Completed Features ✅

### Phase 1 - Core Usability
- **Theme Support** - Dark/light mode toggle with persistence
- **Compact File Tree** - Scalable font sizing for better space utilization
- **Path Information** - Full paths in tooltips, parent folder display
- **File Safety** - Verified read-only access (safe for production)
- **Explorer Integration** - Open file locations in system file manager
- **Paused Files** - YAML support for starting busy files paused

## Upcoming Features 🎯

### Immediate Priority - Filtering System
1. **Preview Pane Filter**
   - Live filtering of lines in tail preview
   - Regex support
   - Highlight matches
   - Filter persistence per file

2. **File List Filter**
   - Filter visible files in the tree
   - Quick show/hide by name pattern
   - Group filtering

3. **Log Level Filtering**
   - Exclude verbose levels (DEBUG, TRACE)
   - Conditional filtering (e.g., "show DEBUG only if contains 'error'")
   - Min/max level settings

### Events System 🚨
A dedicated tab for monitoring important events across all tailed files.

#### Event Triggers Configuration
```yaml
events:
  - name: "Errors"
    pattern: "ERROR|FATAL|Exception"
    severity: error
    color: "#FF0000"
    alert: true
    
  - name: "Warnings" 
    pattern: "WARN|Warning"
    severity: warning
    color: "#FFA500"
    
  - name: "Payment Failed"
    pattern: "payment.*failed|transaction.*declined"
    severity: critical
    color: "#FF00FF"
    notification: desktop  # Future: desktop notifications
```

#### Event Features
- Timestamp, source file, matched line
- Severity-based sorting
- Clear/export functionality
- Sound alerts for critical events
- Event counting and statistics

### Colorized Log Rendering 🎨

#### Auto-Detection of Log Levels
Detect common patterns:
- `[INFO]`, `[WARN]`, `[ERROR]`
- `INFO:`, `WARN:`, `ERROR:`
- `<info>`, `<warn>`, `<error>`
- ISO timestamps
- Thread IDs

#### Default Color Scheme
- **TRACE/DEBUG**: Dim blue/gray
- **INFO**: Default text color
- **WARN**: Yellow/orange
- **ERROR**: Red
- **FATAL/CRITICAL**: Bright red/magenta

#### YAML Configuration
```yaml
log_format:
  pattern: "\\[(\\w+)\\]"  # Matches [LEVEL]
  colors:
    INFO: "#808080"
    WARN: "#FFA500"
    ERROR: "#FF0000"
```

### File Color Coding
```yaml
files:
  - path: "/var/log/app.log"
    name: "App Log"
    color: "#00FF00"  # Green in file list
    icon: "🚀"        # Optional emoji
```

### Advanced Filtering

#### Exclusion Patterns
```yaml
filters:
  exclude:
    - pattern: "DEBUG|TRACE"
      unless: "important|critical"
    - pattern: "heartbeat|ping"
      
  include_only:
    - pattern: "ERROR|WARN|transaction"
```

#### Dynamic Level Adjustment
- UI slider to change minimum log level
- Per-file level settings
- Quick toggle buttons for common levels

### Layout and Organization

1. **Layout Persistence**
   - Save window splits
   - Remember filter states
   - Export/import layouts

2. **File Grouping**
   - Drag & drop reorganization
   - Dynamic group creation
   - Smart grouping by path patterns

3. **Multi-Monitor Support**
   - Detachable panels
   - Multiple window instances
   - Synchronized filtering

### Performance Features

1. **Smart Buffering**
   - Circular buffers per file
   - Memory limits
   - Automatic old line removal

2. **Virtual Scrolling**
   - Handle millions of lines
   - Smooth performance
   - Jump to timestamp

3. **Background Processing**
   - Separate thread for file reading
   - Pattern matching in background
   - Non-blocking UI

### Integration Features

1. **Export Capabilities**
   - Export filtered results
   - CSV/JSON formats
   - Time range selection

2. **Search History**
   - Recent searches
   - Saved search patterns
   - Search pattern sharing

3. **Alerting**
   - Email notifications
   - Webhook integration
   - Custom scripts on events

## Implementation Priority

### Phase 2 (Current)
1. Basic filter functionality
2. File color support in YAML
3. Colorized log level rendering

### Phase 3
1. Events system with pattern matching
2. Advanced filtering (exclusions, conditionals)
3. Begin modular refactoring

### Phase 4
1. Layout persistence
2. Export capabilities
3. Performance optimizations

### Phase 5
1. Integration features
2. Multi-monitor support
3. Advanced alerting

## Technical Considerations

### Refactoring Approach
- Extract features into modules as we implement
- Keep backward compatibility
- Comprehensive testing for each feature
- Performance benchmarks for filtering

### UI/UX Principles
- Immediate feedback (live filtering)
- Intuitive controls
- Keyboard shortcuts for power users
- Minimal performance impact

### Configuration Philosophy
- YAML for complex configs
- UI for common settings
- Sensible defaults
- Migration support between versions