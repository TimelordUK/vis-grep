# Tail Preview Filter Design

## Overview

The tail preview filter provides a lightweight, in-memory filtering mechanism for the preview pane in tail mode. Unlike grep mode which searches files on disk, this filter operates on already-loaded content in the preview pane.

## Key Differences from Grep Mode

1. **Scope**: Only affects the preview pane, not the main tail output
2. **Highlighting vs Hiding**: Highlights matching lines but shows all content (grep hides non-matches)
3. **Real-time**: Instant filtering as you type
4. **Temporary**: Filter is cleared when switching files or modes

## Features

### 1. Quick Filter Activation
- Press `/` to activate filter input
- Press `Esc` to clear filter and hide input
- Filter input appears at the top of preview pane

### 2. Highlighting
- Matching lines are highlighted with a background color
- Match text within lines is highlighted with a different color
- Non-matching lines remain visible but dimmed

### 3. Navigation
- `n` - jump to next match
- `N` - jump to previous match
- Shows match count: "3 of 15 matches"

### 4. Filter Modes
- **Simple text** (default): Case-insensitive substring match
- **Case-sensitive**: Prefix with `C:` (e.g., `C:ERROR`)
- **Regex**: Prefix with `R:` (e.g., `R:\d{3}-\d{4}`)

## Implementation Details

### Data Structure
```rust
struct PreviewFilter {
    active: bool,
    query: String,
    case_sensitive: bool,
    use_regex: bool,
    match_lines: Vec<usize>,  // Line indices that match
    current_match: Option<usize>,  // Current match index for navigation
}
```

### UI Flow
1. User presses `/` → Filter input appears
2. As user types → Content is filtered in real-time
3. Matching lines are highlighted
4. User can navigate matches with `n`/`N`
5. Press `Esc` → Filter cleared, normal view restored

## Integration with Existing Features

- Filter persists when preview is paused/following
- Filter is cleared when selecting a different file
- Filter state is independent of grep mode search

## Future Enhancements

1. **Multi-highlight**: Support multiple simultaneous filters with different colors
2. **Filter history**: Remember recent filters
3. **Export matches**: Copy filtered lines to clipboard
4. **Inverse filter**: Show only non-matching lines (like `grep -v`)