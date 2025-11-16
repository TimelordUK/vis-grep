# Next Session: Less Mode Implementation

## Overview
Create a new "Less Mode" - a memory-efficient file viewer similar to `less` but with integrated search/filter capabilities and bookmarks.

## Core Requirements

### 1. Memory-Mapped File Viewing
- **Memory mapping** for huge files (GB+)
- **Viewport rendering** - only render visible lines
- Fast navigation through large files
- Bookmarks (ma/'a) that work across the file

### 2. Integrated Filtering
- **Fast filtering** using ripgrep/custom filter
- Filter results written to **temp file** (also memory-mapped)
- Quick toggle between full file and filtered view
- Keep context around matches

### 3. Mode Integration
- **New tab**: "Less" mode alongside Grep/Tail
- **Nominate from Tail**: Right-click or hotkey to send file to Less mode
- Independent viewer state per file

## Architecture Considerations

### Viewport Abstraction
```rust
struct Viewport {
    /// Memory-mapped file handle
    mmap: Mmap,

    /// Current window into file (line numbers)
    visible_range: Range<usize>,

    /// Total lines in file
    total_lines: usize,

    /// Line index (for fast random access)
    line_offsets: Vec<u64>,
}
```

### Filter Pipeline
```
Original File (mmap)
  → ripgrep filter
    → Temp file (mmap)
      → Viewport renders filtered results
```

### Performance Goals
- Handle 10GB+ files smoothly
- Sub-second filtering with ripgrep
- Instant bookmark navigation
- Smooth scrolling (render only visible lines)

## Future Enhancements (Later Sessions)

### Time-Synchronized Viewing
- **Split view**: Two files side-by-side
- **Timestamp parsing**: Extract timestamps from log lines
- **Sync scroll**: Both files move together by time, not line number
- Use case: Compare application logs with database logs at same time

### Diff/Merge View
- Show differences between two files
- Merge changes interactively
- Could build on time-sync infrastructure

## Implementation Plan (Next Session)

### Phase 1: Basic Less Mode
1. Create `LessMode` enum variant
2. Add "Less" tab to UI
3. Implement memory-mapped file loading
4. Build line index for random access
5. Implement viewport with BufferWindow-like abstraction
6. Basic navigation (j/k, gg/G, :goto)

### Phase 2: Viewport Rendering
1. Only render visible lines (viewport optimization)
2. Efficient line range calculation
3. Smooth scrolling with large files
4. Line number gutter

### Phase 3: Bookmarks
1. Integrate BookmarkManager (already built!)
2. ma/'a navigation
3. Visual bookmark indicators in gutter

### Phase 4: Integrated Filtering
1. `/` to activate filter (like grep search)
2. Run ripgrep on memory-mapped file
3. Write results to temp file
4. Memory-map filtered results
5. Toggle between filtered/unfiltered view
6. `n/N` to navigate filter matches

### Phase 5: Integration
1. "Open in Less" from tail mode
2. "Open in Less" from grep results
3. Preserve bookmarks across mode switches

## Technical Notes

### Memory Mapping
- Use `memmap2` crate
- Handle line endings properly (LF vs CRLF)
- Build line index on first load (cache it?)

### Filtering Strategy
- Run: `rg --line-number <pattern> <file> > /tmp/vis-grep-filter-*.txt`
- Parse line numbers to maintain file position mapping
- Keep context lines (e.g., -C 2 for before/after context)

### Line Indexing
- Build on first load: scan file, record byte offset of each line
- Cache to disk for large files?
- Trade-off: memory for line_offsets vs scan time

## Open Questions
1. Should we cache line indices to disk for huge files?
2. How much context to show around filtered lines?
3. Should filtered view show line numbers from original file?
4. UI for toggling filtered/unfiltered view?
5. Should we support multiple filtered views (different patterns)?

## Testing Strategy
- Test with small files (100 lines)
- Test with medium files (100K lines)
- Test with large files (10M lines)
- Test with huge files (100M+ lines, GB+)
- Test filtering performance
- Test bookmark navigation across filtered/unfiltered views

## Success Criteria
- ✅ Can view 10GB file smoothly
- ✅ Filter 1GB file in < 1 second
- ✅ Bookmarks work in filtered view
- ✅ Navigation feels instant
- ✅ Memory usage stays reasonable (<500MB for any file size)

---

## Current State (End of This Session)

### Completed
- ✅ BookmarkManager fully integrated with tail mode
- ✅ BufferWindow abstraction for sliding window management
- ✅ Cross-file bookmarks (ma/mb/'a/'b) working
- ✅ Visual bookmark panel
- ✅ Comprehensive logging
- ✅ Clean separation: buffer view vs absolute file positions

### Ready to Build On
- BookmarkManager can be reused in Less mode
- BufferWindow pattern can guide Viewport design
- Input handling (vim-style keys) already working
- Theme and UI infrastructure in place
