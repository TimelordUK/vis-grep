# VisGrep Roadmap

This document tracks potential features and improvements for VisGrep.

## Status Legend
- 🟢 **Implemented** - Feature is complete and merged
- 🟡 **In Progress** - Currently being worked on
- 🔵 **Planned** - Approved and scheduled
- ⚪ **Idea** - Under consideration
- 🔴 **Rejected** - Decided against

---

## Completed Features 🟢

### Vim-style Navigation
- ✅ Basic navigation: `n`, `p`, `gg`, `G`
- ✅ Count prefixes: `3n`, `5p`
- ✅ File navigation: `N`, `P` (next/previous file)
- ✅ File-local jumps: `^`, `$` (first/last match in file)
- ✅ Marks/bookmarks: `ma`, `mb`, `'a`, `'b`

### Search & Display
- ✅ Regex and literal search
- ✅ Case sensitive/insensitive toggle
- ✅ File pattern filtering (glob)
- ✅ File age filtering
- ✅ Results filter
- ✅ Preview panel with context lines
- ✅ Matched line focus panel
- ✅ Pattern highlighting in matched lines

### Clipboard & File Operations
- ✅ Yank matched line: `yy`
- ✅ Open in file explorer: `gf`

### Configuration
- ✅ Folder presets (YAML config)
- ✅ Tilde expansion for paths

---

## Future Ideas ⚪

### 1. Search History
**Priority**: Medium
**Complexity**: Low

Track recent searches and allow quick recall:
- Keep last N searches (query + path + pattern)
- Show in dropdown or sidebar
- Keyboard shortcut to cycle through history
- Persist to config file

**Use case**: Frequently re-running the same searches on different log files

---

### 2. Exclude Patterns
**Priority**: Medium
**Complexity**: Low

Skip certain file patterns during search:
- UI field for exclude patterns (e.g., `*.min.js,*.lock`)
- Multiple patterns separated by comma
- Applied during file walking phase
- Save in config as defaults

**Use case**: Large codebases with generated/vendor files

---

### 3. Multi-pattern Search (OR Logic)
**Priority**: High
**Complexity**: Medium

Search for multiple patterns at once:
- Input multiple queries (one per line or comma-separated)
- Show which pattern matched each result
- Color-code different patterns
- AND vs OR toggle

**Use case**: Finding any of several FIX tag patterns in logs

---

### 4. Export Results
**Priority**: Medium
**Complexity**: Low

Save search results for analysis:
- Export formats: CSV, JSON, plain text
- Include: file path, line number, matched text, pattern
- Button in UI to trigger export
- File dialog to choose save location

**Use case**: Sharing results, importing into Excel/scripts for analysis

---

### 5. Context Lines Configuration
**Priority**: Low
**Complexity**: Low

Currently shows 50 lines before/after. Make configurable:
- Slider or input field for context line count
- Separate before/after counts
- Save preference in config

**Use case**: Fine-tuning preview window size

---

### 6. Date Range Filtering
**Priority**: Medium
**Complexity**: Medium

Filter files by modified date range:
- Currently have "file age in hours"
- Add: from date, to date pickers
- Calendar UI or text input
- Combine with age filter

**Use case**: "Show me logs from last Tuesday" or "Between Jan 1-15"

---

### 7. Bookmarked Searches
**Priority**: High
**Complexity**: Medium

Save complete search configurations:
- Name + path + file pattern + query + options
- Quick-load from dropdown
- Manage (add/edit/delete) in UI
- Store in config YAML

**Use case**: "FIX Order Search", "Error Log Hunt", etc. - one click to load entire search setup

---

### 8. Follow Mode (Live Tail) 🔥
**Priority**: High
**Complexity**: High

Watch multiple files for new content in real-time:
- Like `tail -f` but for multiple files simultaneously
- Activity indicators showing which files are active
- Built-in filtering on live output
- Reliable on Windows network shares (use file size, not mtime)
- Separate "Tail Mode" tab to avoid UI clutter

**Use case**:
- Monitor 20 FIX session logs simultaneously
- See which sessions have activity
- Filter live output for errors or specific patterns
- Better than BareTail + integrated with grep

**Implementation**: File size-based detection (reliable on SMB), polling every 250ms

**Design Doc**: See docs/TAIL_MODE_DESIGN.md for detailed design

---

### 9. Column/Structured View
**Priority**: Medium
**Complexity**: High

Parse structured logs (CSV, TSV, FIX):
- Detect delimiters
- Show in table/column view
- Search specific columns
- Sort by column

**Use case**: FIX messages have key=value structure, would be nice to see as columns

**Note**: This is ambitious - might be better as separate tool

---

### 10. Performance Improvements
**Priority**: Low
**Complexity**: Varies

Ongoing optimization opportunities:
- Incremental search (show results as found)
- Search cancellation
- Memory-mapped file improvements
- Better indexing for large result sets

---

### 11. Regex Pattern Library
**Priority**: Low
**Complexity**: Low

Preset regex patterns for common use cases:
- Email addresses
- IP addresses
- UUIDs
- Timestamps
- FIX tag patterns (e.g., `35=D`, `150=\w`)
- Quick insert from dropdown

**Use case**: Don't have to remember/type complex regex

---

### 12. Syntax Highlighting in Preview
**Priority**: Low
**Complexity**: Medium

Currently have basic highlighting infrastructure:
- Apply to preview panel (not just matched line)
- Support more file types
- Custom color schemes
- Toggle on/off

**Use case**: Easier to read code in preview

---

### 13. Diff View
**Priority**: Low
**Complexity**: High

Compare two search results or files:
- Side-by-side view
- Highlight differences
- Navigate between changes

**Use case**: Comparing FIX messages before/after change

---

### 14. Search within Results
**Priority**: Medium
**Complexity**: Low

Filter current results further:
- Secondary search field
- Narrows down current results
- Can stack multiple filters
- Clear to return to full results

**Use case**: First search for "ERROR", then filter those for specific error codes

---

### 15. Keyboard Shortcuts Help
**Priority**: High
**Complexity**: Low

Built-in help overlay:
- Press `?` to show all keyboard shortcuts
- Categorized (navigation, editing, etc.)
- Searchable
- Maybe print to console on startup

**Use case**: Learning/remembering all the vim commands

---

### 16. Themes/Dark Mode
**Priority**: Low
**Complexity**: Low

Currently using default egui theme:
- Proper dark mode
- Light mode option
- High contrast mode
- Custom color schemes

---

### 17. Split View
**Priority**: Medium
**Complexity**: Medium

View multiple files/matches simultaneously:
- Vertical/horizontal split
- Independent scrolling
- Separate navigation

**Use case**: Comparing two related log files

---

### 18. Search Performance Metrics
**Priority**: Low
**Complexity**: Low

Show stats after search:
- Files scanned
- Matches found
- Time taken
- Data processed (MB/GB)

**Use case**: Understanding search performance

---

### 19. Binary File Handling
**Priority**: Low
**Complexity**: Low

Currently might try to search binary files:
- Detect binary files
- Skip or warn
- Option to search anyway
- Hex view for matches

---

### 20. Remote File Search
**Priority**: Low
**Complexity**: Very High

Search files on remote servers:
- SSH integration
- Remote path browsing
- Stream results back

**Use case**: Searching logs on production servers

**Note**: Probably too complex, better to use SSH + local tool

---

## Rejected Ideas 🔴

### Zip File Searching
**Reason**: Better handled by external tools. User has PowerShell scripts for batch extraction. Adds complexity for marginal benefit.

---

## Notes

- Features marked as "High Priority" are strong candidates for next implementation
- "Low Complexity" items are good for quick wins
- Focus on features that enhance the FIX log analysis workflow
- Keep the tool fast and focused - avoid feature bloat
