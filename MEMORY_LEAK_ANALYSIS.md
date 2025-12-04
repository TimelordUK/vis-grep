# Memory Leak Analysis for vis-grep

## Problem
The application shows continuous memory growth even when idle with all files paused. Memory grows from ~150MB to 600MB+ over time.

## Root Cause
The rendering functions are cloning data structures on EVERY frame (60fps):

1. In `render_tail_file_list()`:
   - Creating new FontId objects every frame
   - Cloning group IDs into new Vec
   - Inserting into style system repeatedly

2. In `render_file_group_by_id()`:
   - Cloning group names, icons
   - Cloning entire file lists
   - Creating new Vecs of child group IDs

This results in ~60 allocations per second per group/file.

## Why This Causes Memory Growth
1. **Heap Fragmentation**: Continuous small allocations fragment memory
2. **egui Internal State**: Style insertions might be retained internally
3. **OS Memory**: The OS doesn't always return freed memory immediately

## Proper Solution
As the egui docs recommend, use Arc to share data between frames:

1. Store group/file data in Arc<T> to avoid cloning
2. Cache UI state that doesn't change often
3. Only update styles when font size actually changes
4. Use persistent IDs for UI elements

## Temporary Workaround
Our style cache helps but doesn't address the core issue of cloning in render_file_group_by_id.

## Next Steps
Need to refactor the data structures to use Arc and avoid per-frame allocations entirely.