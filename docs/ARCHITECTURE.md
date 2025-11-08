# Architecture Overview

## Project Structure

```
vis-grep/
├── src/
│   ├── main.rs          # Main app, UI, event loop
│   ├── search.rs        # Search engine, file walking, pattern matching
│   ├── preview.rs       # File preview loading with context
│   ├── input_handler.rs # Vim-style keyboard input processing
│   ├── config.rs        # YAML config loading/saving
│   └── highlighter.rs   # Syntax highlighting (syntect)
├── docs/
│   ├── ROADMAP.md       # Feature planning and ideas
│   └── ARCHITECTURE.md  # This file
├── CONFIG.md            # User-facing config documentation
└── Cargo.toml           # Dependencies
```

## Core Components

### 1. VisGrepApp (main.rs)
The main application struct and UI rendering.

**Responsibilities:**
- UI layout and rendering (egui)
- Application state management
- Keyboard input handling
- Results display
- Preview panel coordination

**Key State:**
- Search parameters (path, pattern, query, options)
- Search results (`Vec<SearchResult>`)
- Selected result index
- Preview content
- Vim navigation state (marks, pending commands)
- Configuration

### 2. SearchEngine (search.rs)
File system walking and content searching.

**Responsibilities:**
- Walk directory tree (using `walkdir`)
- Filter files by pattern and age
- Search file contents in parallel (using `rayon`)
- Return structured results

**Key Features:**
- Recursive/non-recursive search
- Glob pattern matching
- File age filtering
- Parallel file processing
- No .gitignore filtering (searches everything)

**Performance:**
- Uses rayon for parallel file searching
- Memory-mapped files for large files (in preview)

### 3. FilePreview (preview.rs)
Loads and formats file content with context around matches.

**Responsibilities:**
- Load preview window around target line
- Handle large files efficiently
- Format with line numbers
- Mark matched line with `>>>`

**Strategies:**
- Small files (<10MB): Load entirely via BufReader
- Large files: Memory-mapped reading
- Context window: 50 lines before/after (configurable)

### 4. InputHandler (input_handler.rs)
Processes keyboard input and converts to navigation commands.

**Responsibilities:**
- Multi-key sequence handling (e.g., `gg`, `ma`)
- Count prefix handling (e.g., `3n`)
- State machine for pending keys
- Command validation

**Supported Commands:**
- Navigation: `n`, `p`, `gg`, `G`, `^`, `$`, `N`, `P`
- Marks: `ma-z`, `'a-z`
- Clipboard: `yy`
- File ops: `gf`

### 5. Config (config.rs)
Configuration loading and management.

**Responsibilities:**
- Load YAML config from `~/.config/vis-grep/config.yaml`
- Provide defaults if missing
- Save configuration
- Folder preset management

**Config Structure:**
```rust
struct Config {
    folder_presets: Vec<FolderPreset>,
}

struct FolderPreset {
    name: String,
    path: String,
}
```

### 6. SyntaxHighlighter (highlighter.rs)
Syntax highlighting for preview (currently minimal).

**Responsibilities:**
- Detect file type by extension
- Apply syntax highlighting (using `syntect`)
- Currently mostly used for capability, not fully integrated

**Note:** Highlighting is applied to matched line panel, not full preview yet.

## Data Flow

### Search Flow
```
User Input (UI)
    ↓
VisGrepApp::perform_search()
    ↓
SearchEngine::search()
    ├─→ WalkDir (collect matching files)
    ├─→ Parallel file search (rayon)
    └─→ Return Vec<SearchResult>
    ↓
VisGrepApp::results (update)
    ↓
UI renders results list
```

### Preview Flow
```
User selects result
    ↓
VisGrepApp::select_result()
    ↓
FilePreview::load_file(path, line_num)
    ├─→ Small file: BufReader
    └─→ Large file: mmap
    ↓
FilePreview::content (updated)
    ↓
UI renders preview with highlighting
```

### Navigation Flow
```
Keyboard input (egui)
    ↓
InputHandler::process_input()
    ├─→ Build multi-key commands
    ├─→ Handle count prefixes
    └─→ Return NavigationCommand (optional)
    ↓
VisGrepApp::handle_navigation()
    ├─→ Update selected_result
    ├─→ Reload preview
    └─→ Scroll to result
    ↓
UI updates
```

## Dependencies

### Core
- `eframe` / `egui` - GUI framework (immediate mode)
- `walkdir` - Directory traversal
- `regex` - Pattern matching
- `rayon` - Parallel processing

### Features
- `arboard` - Clipboard operations
- `rfd` - Native file dialogs
- `syntect` - Syntax highlighting
- `memmap2` - Memory-mapped file I/O
- `serde` / `serde_yaml` - Configuration serialization

### Utilities
- `log` / `env_logger` - Logging

## Performance Considerations

### What's Fast
- Parallel file searching (rayon)
- Memory-mapped large files
- Lazy preview loading (only on selection)
- Efficient result filtering

### Potential Bottlenecks
- Very large directory trees (thousands of files)
- Huge files (GB+) in preview
- Complex regex patterns
- No search cancellation yet

## Future Architecture Considerations

### Incremental Search
Could split search into:
1. File collection phase (fast)
2. Content search phase (incremental, cancellable)
3. Stream results to UI as found

### Indexing
For repeated searches on same files:
- Build index of files
- Cache partial results
- Detect file changes

### Plugin System
For extensibility:
- Custom parsers (FIX, CSV, etc.)
- Custom renderers
- Export formats

## Build & Run

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run (with X11 backend on Linux/WSL)
WINIT_UNIX_BACKEND=x11 ./target/release/vis-grep

# Run with logging
RUST_LOG=info ./target/release/vis-grep
```

## Testing Strategy

Currently minimal testing. Future test areas:
- Unit tests for search logic
- Input handler state machine tests
- Config loading/saving tests
- Integration tests for file operations

## Platform Support

- **Linux**: Primary development platform
- **Windows**: Tested and working
- **macOS**: Should work (egui is cross-platform)

### Platform-specific Notes
- WSL: Requires `WINIT_UNIX_BACKEND=x11`
- File path separators handled by `std::path`
- Tilde expansion uses `$HOME` env var
