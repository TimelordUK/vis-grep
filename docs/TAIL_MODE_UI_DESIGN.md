# Tail Mode UI Design - Implementation Plan

## UI Layout

```
┌─────────────────────────────────────────────────────────────────────┐
│ VisGrep              [🔍 Grep Mode] [📄 Tail Mode]    Command: n    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                       │
│ Files Being Monitored:                          [+ Add] [⏸ Pause All]│
│ ┌───────────────────────────────────────────────────────────────────┐│
│ │ ● test_1.log                  2.4 KB  (+120 lines)  [⏸] [×]      ││
│ │ ○ test_2.log                  1.8 KB  (idle)        [⏸] [×]      ││
│ │ ● test_3.log                  3.1 KB  (+45 lines)   [⏸] [×]      ││
│ └───────────────────────────────────────────────────────────────────┘│
│                                                                       │
│ Output:                                   [🔍] Filter  [Clear] [⏸/▶] │
│ ┌───────────────────────────────────────────────────────────────────┐│
│ │ 20:15:32 [test_1.log] INFO  Scheduler - Background job complete  ││
│ │ 20:15:33 [test_3.log] WARN  APIHandler - User 5863 authenticated ││
│ │ 20:15:33 [test_1.log] DEBUG Database - Cache hit ratio: 72%      ││
│ │ 20:15:34 [test_3.log] INFO  WebServer - Received 10 messages     ││
│ │ ...                                                                ││
│ │                                                                    ││
│ │ [Auto-scroll ✓]                         Showing 847 / 10000 lines││
│ └───────────────────────────────────────────────────────────────────┘│
│                                                                       │
│ Files: 3  Active: 2  Total lines: 847  Buffer: 8.5%                 │
└─────────────────────────────────────────────────────────────────────┘
```

## Data Structures

### TailedFile (per-file state)

```rust
struct TailedFile {
    // Identity
    path: PathBuf,
    display_name: String,  // Short name for UI

    // File monitoring
    file_handle: Option<File>,  // Keep handle open
    last_size: u64,
    last_position: u64,
    last_modified: SystemTime,

    // Activity tracking
    is_active: bool,  // Currently receiving updates
    last_activity: Instant,
    lines_since_last_read: usize,

    // Throttling
    paused: bool,
    throttle_state: ThrottleState,
    bytes_per_second: f64,  // Recent throughput

    // Statistics
    total_lines_read: usize,
    total_bytes_read: u64,
}

enum ThrottleState {
    Normal,
    Throttled { skip_ratio: f32 },  // e.g., 0.5 = skip 50% of lines
    Paused { reason: ThrottleReason },
}

enum ThrottleReason {
    TooFast,        // Automatic throttle due to speed
    UserPaused,     // User manually paused
    BufferFull,     // Output buffer at capacity
}
```

### TailState (global tail mode state)

```rust
struct TailState {
    // Files being monitored
    files: Vec<TailedFile>,
    selected_file_index: Option<usize>,  // For per-file operations

    // Output buffer (circular)
    output_buffer: VecDeque<LogLine>,
    max_buffer_lines: usize,  // e.g., 10,000

    // Global controls
    paused_all: bool,
    auto_scroll: bool,

    // Filtering (future)
    filter_pattern: String,
    filter_regex: Option<Regex>,

    // Polling
    last_poll_time: Instant,
    poll_interval_ms: u64,  // Default: 250ms

    // Statistics
    total_lines_received: usize,
    lines_dropped: usize,  // Due to throttling

    // Performance tuning
    max_lines_per_poll: usize,  // e.g., 100
    throttle_threshold_bps: u64,  // Bytes per second before throttling
}

struct LogLine {
    timestamp: Instant,
    source_file: String,  // Display name
    line_number: usize,   // Line number in source file
    content: String,

    // Future: for filtering
    matches_filter: bool,
}
```

## Update Logic (Polling Loop)

```rust
fn update_tail_mode(&mut self) {
    if self.tail_state.paused_all {
        return;  // Global pause
    }

    let now = Instant::now();
    let elapsed = now.duration_since(self.tail_state.last_poll_time);

    // Poll at configured interval (default 250ms)
    if elapsed < Duration::from_millis(self.tail_state.poll_interval_ms) {
        return;
    }

    self.tail_state.last_poll_time = now;

    // Poll each file
    for file in &mut self.tail_state.files {
        if file.paused {
            continue;
        }

        match check_file_for_updates(file) {
            Ok(new_lines) => {
                if !new_lines.is_empty() {
                    file.is_active = true;
                    file.last_activity = now;
                    file.lines_since_last_read = new_lines.len();

                    // Apply throttling if needed
                    let lines_to_add = apply_throttling(file, new_lines);

                    // Add to output buffer
                    add_lines_to_buffer(&mut self.tail_state, file, lines_to_add);
                } else {
                    // Mark as idle after 2 seconds
                    if now.duration_since(file.last_activity) > Duration::from_secs(2) {
                        file.is_active = false;
                        file.lines_since_last_read = 0;
                    }
                }
            }
            Err(e) => {
                warn!("Error reading {}: {}", file.display_name, e);
            }
        }
    }

    // Trim buffer if over capacity
    while self.tail_state.output_buffer.len() > self.tail_state.max_buffer_lines {
        self.tail_state.output_buffer.pop_front();
        self.tail_state.lines_dropped += 1;
    }
}

fn check_file_for_updates(file: &mut TailedFile) -> Result<Vec<String>> {
    // Get current file size
    let metadata = fs::metadata(&file.path)?;
    let current_size = metadata.len();

    if current_size > file.last_size {
        // File grew - read new content
        let mut f = File::open(&file.path)?;
        f.seek(SeekFrom::Start(file.last_position))?;

        let reader = BufReader::new(f);
        let new_lines: Vec<String> = reader.lines()
            .filter_map(|l| l.ok())
            .collect();

        file.last_size = current_size;
        file.last_position = current_size;
        file.total_bytes_read += (current_size - file.last_position);

        Ok(new_lines)
    } else if current_size < file.last_size {
        // File was truncated/rotated
        file.last_position = 0;
        file.last_size = current_size;
        Ok(vec!["[FILE TRUNCATED/ROTATED]".to_string()])
    } else {
        // No change
        Ok(vec![])
    }
}
```

## Throttling Strategy

### Automatic Throttling Levels

```rust
fn calculate_throttle_level(file: &TailedFile) -> ThrottleState {
    // Calculate recent throughput (bytes/sec)
    let throughput = file.bytes_per_second;

    // Thresholds
    const THROTTLE_THRESHOLD: f64 = 100_000.0;  // 100 KB/s
    const PAUSE_THRESHOLD: f64 = 500_000.0;     // 500 KB/s

    if throughput > PAUSE_THRESHOLD {
        ThrottleState::Paused {
            reason: ThrottleReason::TooFast
        }
    } else if throughput > THROTTLE_THRESHOLD {
        // Skip percentage based on how much over threshold
        let over_ratio = (throughput - THROTTLE_THRESHOLD) / THROTTLE_THRESHOLD;
        let skip_ratio = over_ratio.min(0.9);  // Max 90% skip
        ThrottleState::Throttled { skip_ratio: skip_ratio as f32 }
    } else {
        ThrottleState::Normal
    }
}

fn apply_throttling(file: &TailedFile, lines: Vec<String>) -> Vec<String> {
    match file.throttle_state {
        ThrottleState::Normal => lines,
        ThrottleState::Throttled { skip_ratio } => {
            // Sample lines based on skip ratio
            lines.into_iter()
                .enumerate()
                .filter(|(i, _)| (*i as f32 / lines.len() as f32) > skip_ratio)
                .map(|(_, line)| line)
                .collect()
        }
        ThrottleState::Paused { .. } => vec![],
    }
}
```

## UI Components

### File List Panel

```rust
fn render_file_list(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Files Being Monitored:");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(if self.tail_state.paused_all { "▶ Resume All" } else { "⏸ Pause All" }).clicked() {
                self.tail_state.paused_all = !self.tail_state.paused_all;
            }
            if ui.button("+ Add").clicked() {
                // TODO: File picker dialog
            }
        });
    });

    ui.separator();

    // File list
    egui::ScrollArea::vertical()
        .max_height(150.0)
        .show(ui, |ui| {
            for (idx, file) in self.tail_state.files.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    // Activity indicator
                    let indicator = if file.is_active { "●" } else { "○" };
                    let color = if file.is_active {
                        egui::Color32::from_rgb(0, 255, 0)
                    } else {
                        egui::Color32::GRAY
                    };
                    ui.colored_label(color, indicator);

                    // Filename (selectable for focus)
                    let selected = self.tail_state.selected_file_index == Some(idx);
                    if ui.selectable_label(selected, &file.display_name).clicked() {
                        self.tail_state.selected_file_index = Some(idx);
                    }

                    // Size
                    ui.label(format!("{:.1} KB", file.last_size as f64 / 1024.0));

                    // Activity info
                    if file.is_active {
                        ui.colored_label(
                            egui::Color32::from_rgb(255, 200, 100),
                            format!("(+{} lines)", file.lines_since_last_read)
                        );
                    } else {
                        ui.label("(idle)");
                    }

                    // Throttle indicator
                    match file.throttle_state {
                        ThrottleState::Throttled { skip_ratio } => {
                            ui.colored_label(
                                egui::Color32::YELLOW,
                                format!("⚠ {}%", (skip_ratio * 100.0) as i32)
                            );
                        }
                        ThrottleState::Paused { reason } => {
                            ui.colored_label(egui::Color32::RED, "⏸ PAUSED");
                        }
                        _ => {}
                    }

                    // Individual pause button
                    if ui.small_button(if file.paused { "▶" } else { "⏸" }).clicked() {
                        file.paused = !file.paused;
                    }

                    // Remove button
                    if ui.small_button("×").clicked() {
                        // Mark for removal (handle outside iterator)
                    }
                });
            }
        });
}
```

### Output Panel

```rust
fn render_output_panel(&mut self, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Output:");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(if self.tail_state.paused_all { "▶" } else { "⏸" }).clicked() {
                self.tail_state.paused_all = !self.tail_state.paused_all;
            }
            if ui.button("Clear").clicked() {
                self.tail_state.output_buffer.clear();
            }

            // Future: filter button
            ui.label("🔍");
            ui.add(egui::TextEdit::singleline(&mut self.tail_state.filter_pattern)
                .hint_text("Filter...")
                .desired_width(150.0));
        });
    });

    ui.separator();

    // Output area with auto-scroll
    let available_height = ui.available_height() - 60.0;

    let mut scroll_area = egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .max_height(available_height)
        .stick_to_bottom(self.tail_state.auto_scroll);

    scroll_area.show(ui, |ui| {
        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

        for log_line in &self.tail_state.output_buffer {
            // Color-code by source file
            let color = get_color_for_file(&log_line.source_file);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                // Timestamp
                let elapsed = log_line.timestamp.elapsed();
                let time_str = format!("{:02}:{:02}:{:02}",
                    elapsed.as_secs() / 3600,
                    (elapsed.as_secs() % 3600) / 60,
                    elapsed.as_secs() % 60
                );
                ui.label(egui::RichText::new(time_str).color(egui::Color32::GRAY));

                // Source file
                ui.colored_label(color, format!("[{}]", log_line.source_file));

                // Content
                ui.label(&log_line.content);
            });
        }
    });

    // Status bar
    ui.horizontal(|ui| {
        ui.checkbox(&mut self.tail_state.auto_scroll, "Auto-scroll");

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let buffer_pct = (self.tail_state.output_buffer.len() as f32
                / self.tail_state.max_buffer_lines as f32) * 100.0;
            ui.label(format!("Showing {} / {} lines (buffer: {:.1}%)",
                self.tail_state.output_buffer.len(),
                self.tail_state.max_buffer_lines,
                buffer_pct
            ));
        });
    });
}
```

## Color Coding Strategy

```rust
fn get_color_for_file(filename: &str) -> egui::Color32 {
    // Deterministic color based on filename hash
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    filename.hash(&mut hasher);
    let hash = hasher.finish();

    // Generate pleasant, distinguishable colors
    let hue = (hash % 360) as f32;
    let saturation = 0.7;
    let lightness = 0.6;

    // Convert HSL to RGB
    hsl_to_rgb(hue, saturation, lightness)
}
```

## Configuration

Add to `config.yaml`:

```yaml
tail_mode:
  poll_interval_ms: 250
  max_buffer_lines: 10000
  max_lines_per_poll: 100
  throttle_threshold_bps: 100000  # 100 KB/s
  pause_threshold_bps: 500000     # 500 KB/s
  auto_scroll_default: true
  color_by_file: true
```

## Future Enhancements (Post-MVP)

1. **Filtering/Search**: Regex filter on live output
2. **File Organization**: Tree view with folders/categories
3. **Highlighting**: Highlight specific patterns (errors, warnings)
4. **Export**: Save filtered output to file
5. **Bookmarks**: Mark interesting log lines
6. **Split View**: Show multiple files in separate panes
7. **Statistics**: Charts/graphs of activity over time
8. **Follow Rotation**: Automatically follow log rotation (file.log → file.log.1)

## Implementation Order

1. ✅ Basic TailedFile and TailState structures
2. ✅ File size monitoring and change detection
3. ✅ Circular buffer for log lines
4. ✅ File list UI with activity indicators
5. ✅ Output panel with color coding
6. ✅ Pause/resume controls
7. ✅ Throttling for fast files
8. ⏳ Testing with test harness
9. 🔮 Filtering (next iteration)
10. 🔮 File organization (future)
