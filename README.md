# VisGrep - Fast Visual Search Tool

A modern, fast, native GUI for searching code and log files. Think BareGrep for the modern age.

## Features

- **Blazing Fast**: Built with Rust, uses parallel search with rayon
- **Smart File Preview**: Memory-mapped file reading for instant previews, even with GB-sized files
- **Regex Support**: Full regex search capabilities
- **File Filtering**: Glob patterns to filter which files to search (e.g., `*.log`, `*.messages*.log`)
- **Recursive Search**: Search through entire directory trees
- **Lightweight**: Native GUI using Dear ImGui - no Electron bloat
- **Cross-Platform**: Works on Linux and Windows

## Why VisGrep?

If you constantly use `rg | less`, search, exit, `rg` again in a loop - this tool is for you. It provides:
- Instant re-search without retyping commands
- Visual preview of matches with context
- Quick navigation between results
- All the speed of ripgrep with a productive UI

## Building

### Linux

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install libxcb-shape0-dev libxcb-xfixes0-dev

# Build
cargo build --release
```

### Windows

```bash
cargo build --release
```

## Usage

1. **Search Path**: Point to a folder (network or local)
2. **File Pattern**: Filter files (e.g., `*.log`, `*.messages*.log`, `*` for all)
3. **Search Query**: Enter regex or plain text
4. **Options**:
   - Case Sensitive: Enable/disable case sensitivity
   - Regex: Toggle between regex and literal text search
   - Recursive: Search subdirectories

5. Click **Search** or press **Enter** to search
6. Click any result to see a preview with context lines
7. Results show filename, line number, and matching text

## Performance

- Uses parallel search (rayon) across multiple files
- Memory-mapped file reading for large files (> 10MB)
- Only loads preview context (40 lines) instead of entire files
- Native rendering via OpenGL - minimal memory footprint

## Architecture

```
┌─────────────────┐
│   Dear ImGui    │  ← Simple immediate-mode UI (easy to modify)
├─────────────────┤
│  Search Engine  │  ← Parallel regex search with rayon
├─────────────────┤
│  File Preview   │  ← Memory-mapped preview loading
└─────────────────┘
```

## Future Enhancements

- [ ] Syntax highlighting for code previews
- [ ] Support for archived/zipped logs
- [ ] Double-click to open file in $EDITOR
- [ ] Search history
- [ ] Export results
- [ ] Bookmarks/favorites for common searches
- [ ] Replace functionality

## Tech Stack

- **Rust** - Fast, safe, cross-platform
- **Dear ImGui** - Immediate-mode GUI (trivial to modify)
- **Glium** - OpenGL wrapper
- **Rayon** - Data parallelism
- **Regex** - Fast regex engine
- **Memmap2** - Memory-mapped file I/O
- **WalkDir** - Fast directory traversal

## Why ImGui?

Unlike traditional widget toolkits (Qt, GTK, WPF), ImGui uses immediate-mode rendering:

```rust
// This is all you need for a button:
if ui.button("Search") {
    perform_search();
}
```

No XAML, no component lifecycle, no state management complexity. Perfect for tools where iteration speed matters.

## License

MIT
