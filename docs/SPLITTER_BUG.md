# egui 0.29 Splitter Bug Documentation

## Problem Discovered

**Date:** 2025-01-11  
**egui Version:** 0.29.1  
**Issue:** `TopBottomPanel::resizable(true)` is broken - splitters cannot be dragged

## Symptoms

- Cursor changes to resize icon when hovering over the splitter
- Clicking and dragging does nothing - the splitter won't move
- Affects all `TopBottomPanel` with `.resizable(true)` 
- Does NOT affect `SidePanel::resizable()` (vertical splitters work fine)

## Investigation Process

We tried many "fixes" that didn't work:
1. ❌ Removing `.default_height()` - made splitter completely non-draggable
2. ❌ Changing panel creation order - no effect
3. ❌ Disabling continuous repaint - no effect  
4. ❌ Removing dynamic content - no effect
5. ❌ Removing ScrollAreas - no effect
6. ✅ **Creating custom splitter with manual drag handling - WORKS!**

## Root Cause

egui's built-in resize logic for horizontal splits (TopBottomPanel) appears to be broken in version 0.29. The framework sets the resize cursor but doesn't actually handle the drag interaction properly.

## Solution

Use a custom splitter implementation based on:
https://gist.github.com/mkalte666/f9a982be0ac0276080d3434ab9ea4655

Modified for egui 0.29 API (added `None` parameter to `child_ui()`).

## Examples

- `examples/minimal_splitter.rs` - Demonstrates the BROKEN built-in approach
- `examples/custom_splitter_test.rs` - Demonstrates the WORKING custom approach
- `src/splitter.rs` - Custom splitter implementation used in the app

## Testing

```bash
# Test the broken built-in splitter
cargo run --example minimal_splitter --release

# Test the working custom splitter  
cargo run --example custom_splitter_test --release
```

## Future

If this gets fixed in a future egui version, we can replace the custom splitter with:

```rust
egui::TopBottomPanel::top("controls")
    .resizable(true)
    .default_height(200.0)
    .height_range(100.0..=500.0)
    .show(ctx, |ui| {
        // content
    });
```

But for now (egui 0.29), use the custom `Splitter` instead.

