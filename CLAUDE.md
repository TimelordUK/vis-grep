# vis-grep Project Context

## Overview
vis-grep is a visual grep/tail utility built with Rust and egui. It provides two main modes:
1. **Grep Mode**: Search through files with regex patterns and view results
2. **Tail Mode**: Monitor multiple files in real-time with a tree view

## Tech Stack
- **Language**: Rust
- **GUI Framework**: egui (immediate mode GUI)
- **Key Dependencies**:
  - `eframe`: Window management for egui
  - `rfd`: File dialogs
  - `arboard`: Clipboard access
  - `serde`/`serde_yaml`: Configuration management
  - `log`/`env_logger`: Logging

## Key Files and Structure

### Core Application
- `src/main.rs`: Main application struct `VisGrepApp`, window setup, mode switching, editor integration
  - Contains `open_file_in_editor()` function for opening files in configured editors
  - Handles configuration loading and persistence
  
### Modes
- `src/grep_mode.rs`: Grep mode implementation
  - Search functionality
  - Results display with syntax highlighting
  - File navigation
  
- `src/tail_mode.rs`: Tail mode implementation  
  - Real-time file monitoring
  - Tree view with file groups/folders
  - `render_file_entry()`: Renders individual files with pause, copy path, and open editor buttons
  - `render_tail_file_list()`: Main tree rendering logic

### Search and Filtering
- `src/search.rs`: Core search functionality and regex matching
- `src/filter/`: Filtering system
  - `tree.rs`: Tree view filtering
  - `preview.rs`: Preview pane filtering
  - `state.rs`: Filter state management

### UI Components
- `src/splitter.rs`: Resizable splitter widget for panes
- `src/preview_pane.rs`: File preview functionality

### Configuration
- `src/config.rs`: Configuration structures
  - Editor configuration (command + args)
  - Window preferences
  - Search settings

## Configuration Files
- **Windows**: `%APPDATA%\vis-grep\config.yaml`
- **Linux/macOS**: `~/.config/vis-grep/config.yaml`

Example config:
```yaml
editor:
  command: "code"
  args: ["--goto"]

# Alternative for Notepad++ on Windows:
editor:
  command: "C:\\Program Files\\Notepad++\\notepad++.exe"
  args: []
```

## Recent Changes
- Added "Open in Editor" button (📝) to tree view in tail mode
- Fixed Windows config file location
- Implemented configurable editor support with proper fallback chain:
  1. config.yaml editor settings
  2. VISUAL environment variable
  3. EDITOR environment variable
  4. Platform-specific defaults

## UI Features
- **Tree View Buttons** (tail mode):
  - ⏸/▶: Pause/resume file monitoring
  - 📋: Copy full path to clipboard
  - 📝: Open in configured editor
  
- **Toolbar Buttons**:
  - 🏠 Home: Return to search
  - 📁 Explorer: Open in file explorer
  - 📝 Editor: Open in text editor

## Design Patterns
- Immediate mode GUI with egui
- State stored in main `VisGrepApp` struct
- Separate state structs for each mode (`GrepState`, `TailState`)
- Modular filtering system
- Async file operations for tail mode

## Common Tasks
1. **Add UI elements**: Look for `render_*` methods in mode files
2. **Handle keyboard shortcuts**: Check `handle_keyboard_input` in main.rs
3. **Modify configuration**: Update `Config` struct in config.rs
4. **Add filtering**: Extend filter modules in src/filter/

## Build and Test
```bash
cargo build
cargo run
```

## Gotchas
- Borrowing issues with egui closures - use flags or clone data before closures
- Platform-specific path handling for editor commands
- Configuration file auto-creation on first run