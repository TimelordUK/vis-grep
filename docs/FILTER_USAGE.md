# Vis-Grep Filtering System

## Overview

The vis-grep tail mode now includes powerful filtering capabilities for both the preview pane and the file tree. These filters help you focus on specific content when monitoring multiple log files.

## Preview Pane Filter

### Activation
- Press `/` to activate the filter input in the preview pane
- Press `Escape` to clear the filter and hide the input

### Features
- **Real-time highlighting**: Matches are highlighted as you type
- **Match navigation**: Use `n` to jump to next match, `N` (Shift+n) for previous match
- **Match counter**: Shows "X of Y matches" to track your position
- **Multiple match modes**:
  - Default: Case-insensitive substring search
  - Case-sensitive: Prefix your query with `C:` (e.g., `C:ERROR`)
  - Regular expression: Prefix with `R:` (e.g., `R:\d{3}-\d{4}`)

### Visual Feedback
- Matching lines have a blue background
- Current match (when navigating) has a yellow background
- Actual match text within lines is highlighted in yellow

## File Tree Filter

### Usage
- Type in the filter box above the file list
- Files are filtered in real-time using fuzzy matching
- Click the `×` button to clear the filter

### Fuzzy Matching
The filter uses fuzzy matching where all characters must appear in order:
- `test` matches "Test Log 1", "my_test_file.log", "latest" (contains t-e-s-t)
- `tl1` matches "Test Log 1"
- `app` matches "Application Logs"

### Combined Output Filtering
When the tree filter is active, you can optionally apply it to the combined output pane:
- A checkbox appears next to the filter when active
- Check the box to filter the combined output to only show logs from matching files
- The output header changes to "Output (Filtered)" in orange when filtering is active
- Unchecking the box shows all output again while keeping the tree filtered

### Behavior
- Both file paths and display names are searched
- Groups automatically hide when all their files are filtered out
- Groups reappear when files match the filter
- When output filtering is enabled:
  - Only log lines from visible files appear in the combined output
  - Empty state shows "No output from filtered files" if logs exist but are filtered out
  - Helps focus on specific subsystems or files during debugging

## Keyboard Shortcuts Summary

### Preview Pane
- `/` - Activate filter
- `Escape` - Clear filter
- `n` - Next match
- `N` - Previous match
- `j`/`k` - Normal scrolling (still works with filter active)

### General Navigation
- Click on files to select for preview
- Use tree expand/collapse buttons as normal
- Font size controls work with filtering

## Implementation Notes

The filtering system is modular and non-invasive:
- Filters overlay existing functionality without breaking features
- Preview content updates maintain filter state
- File tree structure is preserved during filtering
- Performance optimized for real-time updates

## Future Enhancements

Planned features include:
- Exclude patterns to complement pause functionality
- Filter persistence between sessions
- Multi-highlight with different colors
- Export filtered results
- Filter history