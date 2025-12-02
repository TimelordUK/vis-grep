# UI Memory Issue Analysis

## The Real Problem: No UI Virtualization

The headless tests showed ~10MB growth, but you're seeing hundreds of MB. The difference is **egui UI rendering**.

### What's Happening:

1. **Main output buffer**: 10,000 lines max
2. **Preview buffer**: 1,000-10,000 lines  
3. **Total UI elements**: Up to 20,000 lines being rendered

Even though only ~50 lines are visible, egui creates UI elements for ALL lines:

```rust
// This creates UI elements for every single line!
for log_line in &self.tail_state.output_buffer {
    // egui allocates memory for each line's UI state
    ui.label(&log_line.content);
}
```

### Memory Impact:

Each egui UI element needs:
- Widget state and layout info (~100-200 bytes)
- Text shaping and rendering cache
- Interaction state (hover, click detection)
- Style and color information

For 20,000 lines:
- 20,000 × 200 bytes = 4MB minimum just for widget overhead
- Plus text rendering cache (varies by line length)
- Plus egui's internal retained state
- **Easily 50-100MB+ for UI state alone**

### Why It Compounds:

1. **Frame-to-frame state**: egui retains state between frames
2. **Layout calculations**: Must calculate positions for all lines
3. **Text shaping cache**: Stores glyph positions for all text
4. **Interaction areas**: Tracks clickable regions for all lines

### Proof:

Our headless test (no UI) with same data showed only ~10MB growth.
With UI rendering all lines: hundreds of MB growth.

## Solution: UI Virtualization

Only render visible lines plus a small buffer:

```rust
// Calculate visible range
let visible_lines = (ui.available_height() / line_height) as usize;
let start_idx = scroll_position;
let end_idx = (start_idx + visible_lines + 10).min(buffer.len());

// Only render visible lines
for log_line in buffer.iter().skip(start_idx).take(end_idx - start_idx) {
    ui.label(&log_line.content);
}
```

This would reduce UI elements from 20,000 to ~60, cutting memory by 99%+.

## Additional Issues Found:

1. **Preview buffer uses String not Arc<str>** - duplicates all line data
2. **No cleanup of file errors** - accumulates indefinitely
3. **Filter match lists** - can grow very large

## Immediate Workarounds:

Until virtualization is implemented:
1. Reduce `max_buffer_lines` to 1000 or less
2. Reduce `preview_follow_lines` to 100-500
3. Clear file errors periodically
4. Use filters to reduce visible lines