# Memory Profiling Analysis

## Executive Summary

The memory usage issue in vis-grep's tail mode is primarily caused by **non-virtualized rendering** in the TextViewer widget. The application renders ALL lines in the buffer regardless of visibility, causing massive egui UI state allocation.

### Key Finding
- **Problem**: With 10 files × 500 lines = 5,000 lines total, ALL lines are rendered every frame
- **Impact**: ~400MB memory usage with only ~40 lines visible (< 1% efficiency)
- **Root Cause**: No viewport virtualization in `TextViewer::show()` method

## Memory Profiling Tools Added

### 1. Enhanced Memory Monitor
Added line rendering statistics to the existing `MemoryMonitor`:
- `lines_rendered`: Total lines being processed by egui
- `visible_lines_estimate`: Estimated visible lines in viewport
- `rendering_efficiency()`: Percentage of visible vs rendered lines

### 2. Debug Overlay
- Toggle with `Ctrl+Shift+D`
- Shows real-time memory usage and rendering statistics
- Warns when efficiency drops below 10%

## Technical Analysis

### The Problem Code
In `src/widgets/text_viewer.rs:162-200`:
```rust
for (line_idx, line) in self.content.iter().enumerate() {
    // Renders EVERY line, even if not visible
    let response = filter::preview::render_filtered_line(...);
}
```

### Memory Impact Breakdown
1. **egui UI State**: Each rendered line creates:
   - Label widget state
   - Layout calculations
   - Interaction state (hover, click detection)
   - Text shaping and measurement

2. **String Allocations**: While strings are cached, egui still:
   - Processes text layout for every line
   - Calculates bounding boxes
   - Maintains interaction regions

3. **Multiplication Effect**: With multiple files:
   - 10 files × 500 lines × UI state per line = massive overhead
   - Most of this state is for invisible content

## Recommended Solution: Line Virtualization

### Implementation Strategy
1. **Calculate Visible Range**:
   ```rust
   let viewport_height = scroll_area.inner_rect.height();
   let line_height = font_size + padding;
   let visible_start = (scroll_offset / line_height) as usize;
   let visible_end = visible_start + (viewport_height / line_height).ceil() as usize;
   ```

2. **Render Only Visible Lines**:
   ```rust
   // Add spacer for lines above viewport
   ui.add_space(visible_start as f32 * line_height);
   
   // Render only visible lines
   for line_idx in visible_start..visible_end.min(content.len()) {
       render_line(ui, &content[line_idx], line_idx);
   }
   
   // Add spacer for lines below viewport
   let remaining = content.len().saturating_sub(visible_end);
   ui.add_space(remaining as f32 * line_height);
   ```

### Expected Benefits
- Memory usage reduction: ~90-95% for large buffers
- Rendering performance: O(visible_lines) instead of O(total_lines)
- Maintains smooth scrolling and all functionality

## Testing the Profiler

1. Run vis-grep in tail mode with multiple files
2. Press `Ctrl+Shift+D` to enable debug overlay
3. Observe:
   - Memory usage (current, peak, growth)
   - Lines rendered vs visible
   - Rendering efficiency percentage

## Next Steps

1. **Immediate**: The line virtualization should be implemented in `TextViewer::show()`
2. **Future**: Consider similar optimizations for:
   - Tree view in file list (if it grows large)
   - Search results rendering in grep mode
   - Any other unbounded list rendering

## Configuration Recommendations

Until virtualization is implemented:
- Keep buffer sizes reasonable (500-1000 lines max)
- Limit number of tailed files
- Use the pause feature when not actively monitoring

The memory profiling infrastructure is now in place to verify the improvements once virtualization is implemented.