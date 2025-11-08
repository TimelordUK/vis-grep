# Code Quality Review

## Current State Analysis

### File Sizes
```
1212 lines - src/main.rs       ⚠️ Getting large
 324 lines - src/input_handler.rs  ✅ Good
 172 lines - src/search.rs         ✅ Good
 147 lines - src/config.rs         ✅ Good
 142 lines - src/preview.rs        ✅ Good
  62 lines - src/highlighter.rs    ✅ Good
```

### Main Issues

#### 1. 🔴 **CRITICAL: `update()` function is 386 lines**
**Location**: `src/main.rs:144-530`

This is the UI rendering function and it's doing EVERYTHING:
- Header rendering
- Search controls
- File age filter
- Results panel (with complex scrolling logic)
- Matched line panel
- Preview panel
- Keyboard handling
- Status bar

**Impact**:
- Hard to test
- Hard to understand
- Difficult to modify
- Will get worse when we add Tail Mode

**Priority**: HIGH - Must refactor before adding Tail Mode

#### 2. ⚠️ **Moderate: Multiple similar navigation functions**
**Location**: `src/main.rs:561-943`

We have 10+ similar navigation functions:
- `select_next_match()`
- `select_previous_match()`
- `select_first_match()`
- `select_last_match()`
- `select_first_match_in_current_file()`
- `select_last_match_in_current_file()`
- `select_next_file()`
- `select_previous_file()`

These share a lot of logic and could potentially be consolidated.

**Impact**: Medium - Code duplication, but functions are small and clear

**Priority**: MEDIUM - Could refactor but not urgent

#### 3. ⚠️ **Pattern dropdown logic in update()**
**Location**: `src/main.rs:250-304`

The saved patterns dropdown rendering is inline in `update()`. This is 50+ lines of category grouping and rendering logic.

**Priority**: MEDIUM - Extract to helper function

#### 4. ✅ **Good Separation**
These modules are well-structured:
- `input_handler.rs` - Clean state machine
- `search.rs` - Single responsibility
- `config.rs` - Simple YAML loading
- `preview.rs` - Focused on file preview

## Recommended Refactoring Plan

### Phase 1: Extract UI Components (Before Tail Mode)

#### 1.1 Create `ui` module
```
src/
  ui/
    mod.rs
    search_controls.rs   - Search query, patterns, file filters
    results_panel.rs     - Results list rendering
    preview_panel.rs     - Matched line + preview
    header.rs            - App header with status
```

#### 1.2 Break down `update()` function

**Current**:
```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // 386 lines of UI code...
}
```

**Refactored**:
```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    if let Some(command) = self.input_handler.process_input(ctx) {
        self.handle_navigation_command(command);
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        self.render_header(ui);
        ui.separator();

        self.render_search_controls(ui);
        ui.separator();

        self.render_results_and_preview(ui);
    });

    // Debounced search handling
    self.handle_debounced_search();
    ctx.request_repaint();
}

fn render_header(&mut self, ui: &mut egui::Ui) {
    // 20-30 lines
}

fn render_search_controls(&mut self, ui: &mut egui::Ui) {
    // Search query field + patterns dropdown
    self.render_search_query_field(ui);
    // Search path + folder presets
    self.render_search_path_field(ui);
    // File pattern and filters
    self.render_file_filters(ui);
}

fn render_results_and_preview(&mut self, ui: &mut egui::Ui) {
    // Split panel logic
    self.render_results_panel(ui, available_height);
    self.render_matched_line_panel(ui);
    self.render_preview_panel(ui);
}
```

**Benefits**:
- Each function < 50 lines
- Clear responsibility
- Easy to test
- Ready for Tail Mode tab

### Phase 2: Consolidate Navigation (Optional)

Could create a navigation helper:

```rust
enum NavigationDirection {
    Next,
    Previous,
    First,
    Last,
}

enum NavigationScope {
    Match,
    File,
    MatchInFile,
}

fn navigate(&mut self, direction: NavigationDirection, scope: NavigationScope, count: usize) {
    // Consolidated logic
}
```

**But**: Current code is clear and readable. Only refactor if it becomes a maintenance burden.

## Detailed Refactoring: `update()` Function

### Extract 1: `render_header()`
**Lines**: 151-167
**Size**: ~20 lines
```rust
fn render_header(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("VisGrep - Fast Search Tool");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let status = self.input_handler.get_status();
            if !status.is_empty() {
                ui.label(format!("Command: {}", status));
            }

            if !self.marks.is_empty() {
                let marks_str: String = self.marks.keys().collect();
                ui.label(format!("Marks: {}", marks_str));
            }
        });
    });
}
```

### Extract 2: `render_highlight_pattern_field()`
**Lines**: 170-202
**Size**: ~30 lines

### Extract 3: `render_search_query_field()`
**Lines**: 244-326
**Size**: ~80 lines (includes patterns dropdown)

This should be split further:
- `render_search_query_input()` - 20 lines
- `render_patterns_dropdown()` - 60 lines

### Extract 4: `render_search_path_field()`
**Lines**: 199-234
**Size**: ~35 lines

### Extract 5: `render_file_filters()`
**Lines**: 236-290
**Size**: ~55 lines

### Extract 6: `render_results_panel()`
**Lines**: 339-416
**Size**: ~75 lines

### Extract 7: `render_matched_line_panel()`
**Lines**: 418-423
**Size**: ~5 lines (wrapper for existing function)

### Extract 8: `render_preview_panel()`
**Lines**: 425-474
**Size**: ~50 lines

## Proposed File Structure After Refactor

```
src/
  main.rs                  # App struct, main(), minimal logic

  app/
    mod.rs                 # Re-exports
    state.rs               # VisGrepApp struct definition
    navigation.rs          # All navigation functions
    search.rs              # perform_search, handle_navigation

  ui/
    mod.rs
    header.rs              # Header with status
    search_controls.rs     # All search input controls
    results_panel.rs       # Results rendering
    preview_panel.rs       # Matched line + preview

  input_handler.rs         # Existing - no changes
  search.rs                # Existing - no changes
  preview.rs               # Existing - no changes
  config.rs                # Existing - no changes
  highlighter.rs           # Existing - no changes
```

**Benefits**:
- Each file < 300 lines
- Clear module boundaries
- Easy to add Tail Mode as new module
- Better testability

## Refactoring Strategy

### Option A: Big Refactor Now (Before Tail Mode)
**Pros**:
- Clean foundation for Tail Mode
- Easier to add mode switching
- Better code quality

**Cons**:
- 2-3 hours of refactoring work
- Risk of introducing bugs
- Delays Tail Mode feature

### Option B: Incremental Refactor
**Pros**:
- Start simple - just extract UI helper functions
- Keep working on features
- Refactor as needed

**Cons**:
- Technical debt accumulates
- Harder to test

### Option C: Skip Refactor, Add Tail Mode Carefully
**Pros**:
- Fastest to new feature
- Current code works fine

**Cons**:
- main.rs will grow to 2000+ lines
- `update()` will be unmanageable
- Hard to switch between modes

## Recommendation

### ⭐ **Recommended: Hybrid Approach** ⭐

1. **Minimal Refactor Now** (30 minutes):
   - Extract just the UI rendering functions from `update()`
   - Keep everything in `main.rs` for now
   - Create clear section comments
   - This makes `update()` readable: ~50 lines

2. **Add Tail Mode** (with clean separation):
   - Add `TailState` struct
   - Add `render_tail_mode()` function
   - Mode switching in `update()` becomes simple:
   ```rust
   match self.mode {
       AppMode::Grep => self.render_grep_mode(ui),
       AppMode::Tail => self.render_tail_mode(ui),
   }
   ```

3. **Refactor Later** (if file gets > 2000 lines):
   - Move to module structure
   - Extract to separate files

### Quick Win: Extract UI Functions

**Estimated time**: 30 minutes
**Impact**: High - makes code readable
**Risk**: Low - just moving code to functions

```rust
impl VisGrepApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keyboard input
        if let Some(command) = self.input_handler.process_input(ctx) {
            self.handle_navigation_command(command);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_header(ui);
            ui.separator();
            self.render_search_controls(ui);
            ui.separator();
            self.render_main_content(ui);
            ui.separator();
            self.render_status_bar(ui);
        });

        self.handle_debounced_search();
        ctx.request_repaint();
    }

    // All the extracted functions...
    fn render_header(&mut self, ui: &mut egui::Ui) { /* ... */ }
    fn render_search_controls(&mut self, ui: &mut egui::Ui) { /* ... */ }
    fn render_main_content(&mut self, ui: &mut egui::Ui) { /* ... */ }
    fn render_status_bar(&mut self, ui: &mut egui::Ui) { /* ... */ }
}
```

## Testing Impact

### Current State
- Hard to test UI logic
- No unit tests for rendering
- Manual testing only

### After Refactor
- Smaller functions easier to understand
- Could add integration tests
- Mock UI contexts for testing

## Summary

| Metric | Current | After Extract | After Full Refactor |
|--------|---------|---------------|---------------------|
| `update()` lines | 386 | ~50 | ~30 |
| `main.rs` lines | 1212 | 1212 | ~300 |
| Testability | Low | Medium | High |
| Maintainability | Medium | High | Very High |
| Time to implement | - | 30 min | 3 hours |

## Decision

**What should we do?**

1. ✅ Remove debug logs (DONE)
2. ⚠️ Extract UI functions from `update()` - **YOUR CALL**
3. 🚀 Add Tail Mode with clean separation
4. 🔄 Consider full refactor if complexity grows

The code is currently **functional but needs attention** before major feature additions. A 30-minute extraction of UI functions would make a big difference.
