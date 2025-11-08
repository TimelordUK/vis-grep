# Tail Mode Preview Pane Design

## Overview

Add a preview pane to tail mode that shows the full content of a selected file, similar to `less` or the grep mode preview. This provides context beyond the interleaved output stream.

## UI Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ Files Being Monitored:                          [+ Add] [⏸ Pause All]│
│ ┌───────────────────────────────────────────────────────────────────┐│
│ │ ● test_1.log (SELECTED)      2.4 KB  (+120 lines)  [⏸] [×]      ││
│ │ ○ test_2.log                  1.8 KB  (idle)        [⏸] [×]      ││
│ │ ● test_3.log                  3.1 KB  (+45 lines)   [⏸] [×]      ││
│ └───────────────────────────────────────────────────────────────────┘│
│                                                                       │
│ ┌─────────────────────┬───────────────────────────────────────────┐ │
│ │ Output (Combined):  │ Preview: test_1.log        [📍Follow] [⏸]│ │
│ │ ┌───────────────────┤ ┌─────────────────────────────────────────┤ │
│ │ │ 5s  [test_1.log]  │ │ 1  # Log file header                    │ │
│ │ │ 7s  [test_3.log]  │ │ 2  2025-11-08 20:15:32 [INFO ] Schedu...│ │
│ │ │ 12s [test_1.log]  │ │ 3  2025-11-08 20:15:33 [WARN ] APiHan...│ │
│ │ │ ...               │ │ 4  2025-11-08 20:15:34 [DEBUG] Databa...│ │
│ │ │                   │ │ 5  2025-11-08 20:15:35 [INFO ] WebSer...│ │
│ │ │                   │ │ ...                                      │ │
│ │ │                   │ │ 847 2025-11-08 20:45:12 [INFO ] Schedu│ │ │
│ │ │                   │ │ > (Following - showing last 100 lines) │ │
│ │ └───────────────────┘ └──────────────────────────────────────────┘ │
│ │ [Auto-scroll ✓]     │ Line 847/847  j/k:scroll gg/G:jump       │ │
│ └─────────────────────┴───────────────────────────────────────────┘ │
│ Files: 3  Active: 2  Total lines: 847  Buffer: 8.5%                 │
└─────────────────────────────────────────────────────────────────────┘
```

## Features

### 1. Split View
- Left: Combined output (existing)
- Right: File preview (new)
- Resizable split (optional for later)

### 2. Preview Modes

**Follow Mode (default):**
- 📍 Icon indicator
- Automatically shows last N lines (e.g., 100)
- Updates in real-time as file grows
- Auto-scrolls to bottom like `tail -f`
- Shows "> Following - showing last 100 lines" indicator

**Paused Mode:**
- ⏸ Icon indicator
- User has manually scrolled
- Shows current position: "Line 423/847"
- Vim navigation enabled (j/k, gg/G, Ctrl+D/U)
- Resume follow with: click 📍 button or press 'G'

### 3. Preview State

```rust
struct TailPreviewState {
    // Which file is being previewed
    selected_file_index: Option<usize>,

    // Preview mode
    mode: PreviewMode,

    // Scroll position (for paused mode)
    scroll_offset: f32,

    // Number of lines to show in follow mode
    follow_tail_lines: usize,  // Default: 100

    // Whether to show line numbers
    show_line_numbers: bool,  // Default: true
}

enum PreviewMode {
    Following,  // Track latest, auto-scroll
    Paused,     // Manual navigation
}
```

### 4. File Selection Behavior

When user clicks a file in the list:
1. Set `selected_file_index` to that file
2. Load file content (reuse existing `FilePreview::load_file`)
3. If in Follow mode, scroll to last N lines
4. Highlight the selected file in the list

### 5. Navigation

**Follow Mode:**
- Automatically scrolls to bottom
- Any manual scroll → switch to Paused mode
- Show "📍 Following" button (green)

**Paused Mode:**
- j/k: Scroll line by line
- Ctrl+D/U: Page down/up
- gg: Jump to top
- G: Jump to bottom (and resume following)
- Mouse scroll: Normal scrolling
- Show "⏸ Paused" button (yellow)
- Click button to resume following

### 6. Preview Header

```
Preview: test_1.log (2.4 KB)  [📍 Following] [⏸ Pause]
```

- File name and size
- Follow/Pause toggle button
- Line position indicator

### 7. Preview Footer

```
Line 847/847  j/k:scroll  gg/G:jump  Ctrl+D/U:page
```

## Implementation

### Update TailState

```rust
struct TailState {
    // ... existing fields ...

    // Preview state
    preview_selected_file: Option<usize>,
    preview_mode: PreviewMode,
    preview_scroll_offset: f32,
    preview_follow_lines: usize,
}

enum PreviewMode {
    Following,
    Paused,
}
```

### Update render_tail_mode()

```rust
fn render_tail_mode(&mut self, ui: &mut egui::Ui) {
    // File list (existing)
    self.render_tail_file_list(ui);

    ui.separator();

    // Split into output and preview
    ui.horizontal(|ui| {
        // Left: Combined output (50% width)
        ui.allocate_ui(egui::Vec2::new(ui.available_width() * 0.5, ui.available_height()), |ui| {
            self.render_tail_output(ui);
        });

        ui.separator();

        // Right: File preview (50% width)
        ui.allocate_ui(egui::Vec2::new(ui.available_width(), ui.available_height()), |ui| {
            self.render_tail_preview(ui);
        });
    });

    // Status bar (existing)
}
```

### New: render_tail_preview()

```rust
fn render_tail_preview(&mut self, ui: &mut egui::Ui) {
    if let Some(file_idx) = self.tail_state.preview_selected_file {
        let file = &self.tail_state.files[file_idx];

        // Header
        ui.horizontal(|ui| {
            ui.label(format!("Preview: {} ({:.1} KB)",
                file.display_name,
                file.last_size as f64 / 1024.0
            ));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Pause/Follow toggle
                let (icon, color) = match self.tail_state.preview_mode {
                    PreviewMode::Following => ("📍 Following", egui::Color32::GREEN),
                    PreviewMode::Paused => ("⏸ Paused", egui::Color32::YELLOW),
                };

                if ui.colored_button(icon, color).clicked() {
                    self.tail_state.preview_mode = match self.tail_state.preview_mode {
                        PreviewMode::Following => PreviewMode::Paused,
                        PreviewMode::Paused => PreviewMode::Following,
                    };
                }
            });
        });

        ui.separator();

        // Load file content
        // In Follow mode: load last N lines
        // In Paused mode: load from scroll position

        let available_height = ui.available_height() - 40.0;

        let scroll_area = if self.tail_state.preview_mode == PreviewMode::Following {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
        } else {
            egui::ScrollArea::vertical()
                .scroll_offset(egui::Vec2::new(0.0, self.tail_state.preview_scroll_offset))
        };

        let scroll_output = scroll_area
            .id_salt("tail_preview_scroll")
            .max_height(available_height)
            .show(ui, |ui| {
                // Use egui_extras::syntax_highlighting for nice display
                // Or simple monospace text

                // TODO: Read file content and display
                // If Following: show last N lines
                // If Paused: show all content, scroll to position
            });

        // Detect manual scroll
        if scroll_output.state.offset.y != self.tail_state.preview_scroll_offset {
            // User scrolled manually
            if self.tail_state.preview_mode == PreviewMode::Following {
                self.tail_state.preview_mode = PreviewMode::Paused;
            }
            self.tail_state.preview_scroll_offset = scroll_output.state.offset.y;
        }

        // Footer
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("j/k:scroll  gg/G:jump  Ctrl+D/U:page");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.tail_state.preview_mode == PreviewMode::Following {
                    ui.label(format!("> Following - showing last {} lines",
                        self.tail_state.preview_follow_lines));
                } else {
                    ui.label("Line ?/?");  // TODO: Calculate current line
                }
            });
        });

    } else {
        // No file selected
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("Select a file to preview")
                .italics()
                .color(egui::Color32::GRAY));
        });
    }
}
```

### File Content Reading

For follow mode, read last N lines efficiently:

```rust
fn read_last_n_lines(path: &Path, n: usize) -> Result<Vec<String>, io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines: VecDeque<String> = VecDeque::with_capacity(n);

    for line in reader.lines() {
        if let Ok(line_str) = line {
            if lines.len() >= n {
                lines.pop_front();
            }
            lines.push_back(line_str);
        }
    }

    Ok(lines.into_iter().collect())
}
```

## Keyboard Navigation

Process in `update()` when preview is focused:

```rust
// In tail mode, handle preview navigation
if self.mode == AppMode::Tail {
    ctx.input(|i| {
        if self.tail_state.preview_selected_file.is_some() {
            if i.key_pressed(egui::Key::J) && !i.modifiers.ctrl {
                // Scroll down one line
                self.tail_state.preview_scroll_offset += 20.0;
                self.tail_state.preview_mode = PreviewMode::Paused;
            }
            if i.key_pressed(egui::Key::K) && !i.modifiers.ctrl {
                // Scroll up one line
                self.tail_state.preview_scroll_offset -= 20.0;
                self.tail_state.preview_mode = PreviewMode::Paused;
            }
            if i.key_pressed(egui::Key::G) {
                if i.modifiers.shift {
                    // G - jump to end and resume following
                    self.tail_state.preview_mode = PreviewMode::Following;
                } else {
                    // gg - jump to top
                    self.tail_state.preview_scroll_offset = 0.0;
                    self.tail_state.preview_mode = PreviewMode::Paused;
                }
            }
        }
    });
}
```

## Future Enhancements

1. **Search in preview**: Press `/` to search within file
2. **Copy selection**: Select and copy text from preview
3. **Jump to line**: Press `:` and enter line number
4. **Resizable split**: Drag to resize output vs preview panels
5. **Multiple preview tabs**: Preview multiple files simultaneously
6. **Highlight patterns**: Highlight specific patterns in preview
7. **Follow with offset**: Follow but show N lines above bottom

## Benefits

- **Context**: See full file content, not just interleaved lines
- **Navigation**: Scroll through history with vim keys
- **Flexibility**: Follow mode for live tracking, pause for investigation
- **Familiar**: Works like `less` or grep mode preview
- **Reuse**: Leverages existing `FilePreview` component
