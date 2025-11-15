use crate::{PreviewMode, VisGrepApp, get_color_for_file, filter, log_parser};
use eframe::egui;
use log::info;

impl VisGrepApp {
    pub fn render_tail_mode_controls(&mut self, ui: &mut egui::Ui) {
        
        // Tree filter
        if filter::tree::render_tree_filter(ui, &mut self.tail_state.tree_filter) {
            // Filter changed, we'll handle visibility in the file list rendering
        }
        
        ui.separator();
        
        // File list header
        ui.horizontal(|ui| {
            ui.label("Files Being Monitored:");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(if self.tail_state.paused_all {
                        "▶ Resume All"
                    } else {
                        "⏸ Pause All"
                    })
                    .clicked()
                {
                    self.tail_state.paused_all = !self.tail_state.paused_all;
                }
            });
        });

        ui.separator();

        // Update rate control
        ui.horizontal(|ui| {
            ui.label("Update Rate:");
            
            // Pre-defined rates
            let rates = [
                ("Very Fast", 100),
                ("Fast", 250),
                ("Normal", 500),
                ("Slow", 1000),
                ("Very Slow", 2000),
            ];
            
            for (name, ms) in &rates {
                if ui.selectable_label(self.tail_state.poll_interval_ms == *ms, *name).clicked() {
                    self.tail_state.poll_interval_ms = *ms;
                }
            }
            
            ui.separator();
            ui.label(format!("{} ms", self.tail_state.poll_interval_ms));
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new("(+/- to adjust)")
                        .small()
                        .color(egui::Color32::GRAY)
                );
            });
        });

        ui.separator();

        // Font size control
        ui.horizontal(|ui| {
            ui.label("Font Size:");
            
            // Quick size buttons
            let sizes = [
                ("XS", 10.0),
                ("S", 12.0),
                ("M", 14.0),
                ("L", 16.0),
                ("XL", 18.0),
            ];
            
            for (label, size) in &sizes {
                if ui.selectable_label(self.tail_state.font_size == *size, *label).clicked() {
                    self.tail_state.font_size = *size;
                }
            }
            
            ui.separator();
            
            // Slider for fine control
            ui.add(
                egui::Slider::new(&mut self.tail_state.font_size, 8.0..=24.0)
                    .suffix(" px")
                    .show_value(true)
            );
        });

        ui.separator();

        // File list - use a fixed reasonable max height to avoid feedback loop
        // This prevents the ScrollArea from requesting variable height that causes panel drift
        egui::ScrollArea::vertical()
            .id_salt("file_list_scroll")
            .max_height(300.0) // Fixed height to prevent content from driving panel size
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Add horizontal scrolling for long filenames
                egui::ScrollArea::horizontal()
                    .id_salt("file_list_h_scroll")
                    .show(ui, |ui| {
                        self.render_tail_file_list(ui);
                    });
            });
        
        ui.separator();
        
        // The panels are now handled in main.rs for proper splitter functionality
    }

    fn render_tail_file_list(&mut self, ui: &mut egui::Ui) {
        if self.tail_state.files.is_empty() {
            ui.label("No files being monitored.");
            ui.label("Start with: vis-grep -f /path/to/file.log");
            ui.label("Or load a layout: vis-grep --tail-layout layout.yaml");
        } else if self.tail_state.layout.is_some() {
            // Tree layout mode
            ui.vertical(|ui| {
                // Apply smaller font size for the tree (80% of the main font)
                let tree_font_size = self.tail_state.font_size * 0.8;
                let font_id = egui::FontId::new(tree_font_size, egui::FontFamily::Proportional);
                ui.style_mut().text_styles.insert(egui::TextStyle::Body, font_id.clone());
                ui.style_mut().text_styles.insert(egui::TextStyle::Button, font_id.clone());
                
                // Reduce spacing between items
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.spacing_mut().button_padding.y = 1.0;

                // Calculate maximum filename width for alignment
                let max_filename_len = self.tail_state.files.iter()
                    .map(|f| f.display_name.len())
                    .max()
                    .unwrap_or(0);
                // Approximate character width based on font size
                let char_width = self.tail_state.font_size * 0.6;
                self.tail_state.max_filename_width = (max_filename_len as f32 * char_width).max(100.0) + 20.0;

                // Clone the group IDs to avoid borrow checker issues
                let group_ids: Vec<String> = if let Some(layout) = &self.tail_state.layout {
                    layout.root_groups.iter().map(|g| g.id.clone()).collect()
                } else {
                    Vec::new()
                };
                
                for group_id in group_ids {
                    self.render_file_group_by_id(ui, &group_id, 0);
                }
                
                // Ungrouped files at the end
                let mut has_ungrouped = false;
                for idx in 0..self.tail_state.files.len() {
                    if self.tail_state.files[idx].group_id.is_none() {
                        // Check if file is visible
                        let file = &self.tail_state.files[idx];
                        if filter::tree::is_file_visible(
                            &self.tail_state.tree_filter,
                            &file.path.to_string_lossy(),
                            &file.display_name
                        ) {
                            if !has_ungrouped {
                                has_ungrouped = true;
                                ui.separator();
                                ui.label(egui::RichText::new("Ungrouped Files").strong());
                            }
                            self.render_file_entry(ui, idx, 0);
                        }
                    }
                }
            });
        } else {
            // Flat list mode (original)
            ui.vertical(|ui| {
                // Apply smaller font size for the tree (80% of the main font)
                let tree_font_size = self.tail_state.font_size * 0.8;
                let font_id = egui::FontId::new(tree_font_size, egui::FontFamily::Proportional);
                ui.style_mut().text_styles.insert(egui::TextStyle::Body, font_id.clone());
                ui.style_mut().text_styles.insert(egui::TextStyle::Button, font_id.clone());
                
                // Reduce spacing between items
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.spacing_mut().button_padding.y = 1.0;
                
                for idx in 0..self.tail_state.files.len() {
                    self.render_file_entry(ui, idx, 0);
                }
            });
        }
    }
    
    fn group_has_visible_content(&self, group_id: &str) -> bool {
        if let Some(layout) = &self.tail_state.layout {
            if let Some(group) = layout.find_group(group_id) {
                // Check files
                let has_visible_files = group.files.iter().any(|entry| {
                    if let Some(idx) = entry.tailed_file_idx {
                        if idx < self.tail_state.files.len() {
                            let file = &self.tail_state.files[idx];
                            return filter::tree::is_file_visible(
                                &self.tail_state.tree_filter,
                                &file.path.to_string_lossy(),
                                &file.display_name
                            );
                        }
                    }
                    false
                });
                
                if has_visible_files {
                    return true;
                }
                
                // Check child groups recursively
                return group.groups.iter().any(|child| {
                    self.group_has_visible_content(&child.id)
                });
            }
        }
        false
    }
    
    fn render_file_group_by_id(&mut self, ui: &mut egui::Ui, group_id: &str, depth: usize) {
        // Get group info (cloned to avoid borrow issues)
        let group_info = if let Some(layout) = &self.tail_state.layout {
            if let Some(group) = layout.find_group(group_id) {
                Some((
                    group.name.clone(),
                    group.icon.clone(),
                    group.collapsed,
                    group.has_activity,
                    group.active_file_count,
                    group.total_file_count,
                    group.groups.iter().map(|g| g.id.clone()).collect::<Vec<_>>(),
                    group.files.clone(),
                ))
            } else {
                None
            }
        } else {
            None
        };
        
        if let Some((name, icon, collapsed, has_activity, active_count, total_count, child_group_ids, files)) = group_info {
            // Check if any files in this group are visible
            let has_visible_files = files.iter().any(|entry| {
                if let Some(idx) = entry.tailed_file_idx {
                    if idx < self.tail_state.files.len() {
                        let file = &self.tail_state.files[idx];
                        return filter::tree::is_file_visible(
                            &self.tail_state.tree_filter,
                            &file.path.to_string_lossy(),
                            &file.display_name
                        );
                    }
                }
                false
            });
            
            // Check if any child groups have visible content
            let has_visible_children = child_group_ids.iter().any(|child_id| {
                self.group_has_visible_content(child_id)
            });
            
            // Skip this group if nothing is visible
            if !has_visible_files && !has_visible_children && self.tail_state.tree_filter.active {
                return;
            }
            // Scale indent based on font size (reduced from 20.0 to be more compact)
            let indent = depth as f32 * (self.tail_state.font_size * 1.0);
            
            // Scale row height with font size
            let row_height = self.tail_state.font_size + 2.0; // Minimal padding
            ui.allocate_ui_with_layout(
                egui::Vec2::new(ui.available_width(), row_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                ui.add_space(indent);
                
                // Expand/collapse arrow
                let arrow = if collapsed { "▶" } else { "▼" };
                if ui.small_button(arrow).clicked() {
                    // Toggle collapsed state
                    if let Some(layout) = &mut self.tail_state.layout {
                        if let Some(group) = layout.find_group_mut(group_id) {
                            group.collapsed = !group.collapsed;
                            // Mark as user-controlled to prevent auto-expand
                            group.user_collapsed = Some(group.collapsed);
                        }
                    }
                }
                
                // Group icon
                if let Some(icon) = &icon {
                    ui.label(icon);
                }
                
                // Group name with activity count
                let label = format!("{} ({} active / {} total)", 
                    name, 
                    active_count, 
                    total_count
                );
                
                let color = if has_activity {
                    egui::Color32::from_rgb(200, 255, 200)  // Light green
                } else {
                    ui.style().visuals.text_color()
                };
                
                ui.colored_label(color, label);
                
                // Group controls
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("⏸").on_hover_text("Pause group").clicked() {
                        self.pause_group(group_id);
                    }
                });
            });
            
            // Add minimal spacing between rows
            ui.add_space(1.0);
            
            // Render children if expanded
            if !collapsed {
                // Render subgroups
                for child_id in child_group_ids {
                    self.render_file_group_by_id(ui, &child_id, depth + 1);
                }
                
                // Render files
                for file_entry in &files {
                    // Find the actual file index by matching path
                    if let Some(file_idx) = self.tail_state.files.iter().position(|f| f.path == file_entry.path) {
                        self.render_file_entry(ui, file_idx, depth + 1);
                    }
                }
            }
        }
    }
    
    fn render_file_entry(&mut self, ui: &mut egui::Ui, file_idx: usize, depth: usize) {
        let file = &mut self.tail_state.files[file_idx];
        
        // Check if file should be visible based on filter
        if !filter::tree::is_file_visible(
            &self.tail_state.tree_filter,
            &file.path.to_string_lossy(),
            &file.display_name
        ) {
            return;
        }
        
        // Capture the file path before the closure to avoid borrowing issues
        let file_path = file.path.clone();
        let mut open_in_editor_clicked = false;
        
        // Scale indent based on font size
        let indent = depth as f32 * (self.tail_state.font_size * 1.0);
        
        // Scale row height with font size
        let row_height = self.tail_state.font_size + 2.0; // Minimal padding
        ui.allocate_ui_with_layout(
            egui::Vec2::new(ui.available_width(), row_height),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
            ui.add_space(indent);
            
            // Activity indicator
            let indicator = if file.is_active { "●" } else { "○" };
            let color = if file.is_active {
                egui::Color32::from_rgb(0, 255, 0)
            } else {
                egui::Color32::GRAY
            };
            ui.colored_label(color, indicator);

            // Filename (selectable) - use calculated max width for alignment
            let selected = self.tail_state.preview_selected_file == Some(file_idx);
            let entry_width = self.tail_state.max_filename_width;

            // Extract parent directory for tooltip
            let parent_dir = file.path.parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");

            // Use horizontal with fixed width and clip content
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(entry_width, self.tail_state.font_size + 4.0),
                egui::Sense::click()
            );

            if ui.is_rect_visible(rect) {
                // Save current clip rect
                let old_clip_rect = ui.clip_rect();

                // Clip to the allocated rect
                ui.set_clip_rect(rect.intersect(old_clip_rect));

                let visuals = ui.style().interact_selectable(&response, selected);

                // Only draw background if selected or hovered, otherwise transparent
                let bg_fill = if selected {
                    visuals.bg_fill
                } else if response.hovered() {
                    visuals.bg_fill.linear_multiply(0.3) // Very faint on hover
                } else {
                    egui::Color32::TRANSPARENT // No background normally
                };

                ui.painter().rect(
                    rect,
                    visuals.rounding,
                    bg_fill,
                    visuals.bg_stroke,
                );

                let text_pos = rect.left_center() + egui::vec2(4.0, 0.0);
                ui.painter().text(
                    text_pos,
                    egui::Align2::LEFT_CENTER,
                    &file.display_name,
                    egui::FontId::proportional(self.tail_state.font_size),
                    visuals.text_color(),
                );

                // Restore original clip rect
                ui.set_clip_rect(old_clip_rect);
            }

            if response.clicked() {
                self.tail_state.preview_selected_file = Some(file_idx);
                self.tail_state.preview_needs_reload = true;
                self.tail_state.preview_mode = PreviewMode::Following;
            }

            // Show tooltip with full path and parent directory
            response.on_hover_text(format!(
                "Full path: {}\nDirectory: {}",
                file.path.display(),
                parent_dir
            ));

            // File size - fixed width to prevent jumping
            ui.add_sized(
                egui::vec2(60.0, 20.0),
                egui::Label::new(format!("{:.1} KB", file.last_size as f64 / 1024.0))
            );

            // Activity info - show log level counts if available, otherwise line count
            let (status_text, status_color) = if file.is_active && file.lines_since_last_read > 0 {
                // Check if we have level counts to display
                if !file.level_counts_since_last_read.is_empty() {
                    // Build a compact display of significant log levels
                    let mut parts = Vec::new();

                    // Priority order: FATAL, ERROR, WARN, INFO, DEBUG, TRACE, UNKNOWN
                    use log_parser::LogLevel;
                    let priority_levels = [
                        (LogLevel::Fatal, "FTL"),
                        (LogLevel::Error, "ERR"),
                        (LogLevel::Warn, "WRN"),
                        (LogLevel::Info, "INF"),
                        (LogLevel::Debug, "DBG"),
                        (LogLevel::Trace, "TRC"),
                        (LogLevel::Unknown, "UNK"),
                    ];

                    for (level, abbrev) in &priority_levels {
                        if let Some(count) = file.level_counts_since_last_read.get(level) {
                            if *count > 0 {
                                parts.push(format!("{}:{}", abbrev, count));
                            }
                        }
                    }

                    let text = if parts.is_empty() {
                        format!("(+{} lines)", file.lines_since_last_read)
                    } else {
                        format!("({})", parts.join(" "))
                    };

                    (text, egui::Color32::from_rgb(255, 200, 100))
                } else {
                    (format!("(+{} lines)", file.lines_since_last_read), egui::Color32::from_rgb(255, 200, 100))
                }
            } else if !file.is_active {
                ("(idle)".to_string(), egui::Color32::GRAY)
            } else {
                ("".to_string(), egui::Color32::GRAY)
            };

            ui.add_sized(
                egui::vec2(150.0, 20.0),  // Increased width to accommodate level counts
                egui::Label::new(egui::RichText::new(status_text).color(status_color))
            );

            // Pause button
            if ui.small_button(if file.paused { "▶" } else { "⏸" }).clicked() {
                file.paused = !file.paused;
            }
            
            // Copy path button (small)
            if ui.small_button("📋").on_hover_text("Copy full path").clicked() {
                use arboard::Clipboard;
                match Clipboard::new() {
                    Ok(mut clipboard) => {
                        let path_str = file.path.to_string_lossy().to_string();
                        match clipboard.set_text(&path_str) {
                            Ok(_) => log::info!("Copied path to clipboard: {}", path_str),
                            Err(e) => log::error!("Failed to copy path: {}", e),
                        }
                    }
                    Err(e) => log::error!("Failed to access clipboard: {}", e),
                }
            }
            
            // Open in editor button
            if ui.small_button("📝").on_hover_text("Open in editor").clicked() {
                open_in_editor_clicked = true;
            }
        });
        
        // Handle open in editor outside closure to avoid borrowing issues
        if open_in_editor_clicked {
            self.open_file_in_editor(&file_path);
        }
        
        // Add minimal spacing between rows
        ui.add_space(1.0);
    }
    
    fn pause_group(&mut self, group_id: &str) {
        // Pause all files in the group
        for file in &mut self.tail_state.files {
            if let Some(file_group_id) = &file.group_id {
                if file_group_id == group_id {
                    file.paused = true;
                }
            }
        }
    }
    
    pub fn render_tail_output(&mut self, ui: &mut egui::Ui) {
        // Output header
        ui.horizontal(|ui| {
            // Check if output is filtered
            let is_filtered = self.tail_state.tree_filter.active && 
                             self.tail_state.tree_filter.apply_to_output;
            
            if is_filtered {
                ui.label(
                    egui::RichText::new("Output (Filtered)")
                        .color(egui::Color32::from_rgb(255, 200, 100))
                );
            } else {
                ui.label("Output (Combined):");
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(if self.tail_state.paused_all {
                        "▶"
                    } else {
                        "⏸"
                    })
                    .clicked()
                {
                    self.tail_state.paused_all = !self.tail_state.paused_all;
                }
                if ui.button("Clear").clicked() {
                    self.tail_state.output_buffer.clear();
                    self.tail_state.total_lines_received = 0;
                    self.tail_state.lines_dropped = 0;
                }
            });
        });

        // Log level filter controls
        ui.horizontal(|ui| {
            ui.label("Level:");

            // Cycle through filter modes with buttons
            let current_mode = self.tail_state.log_level_filter.display_mode();

            if ui.selectable_label(current_mode == "ALL", "ALL").clicked() {
                self.tail_state.log_level_filter.active = false;
            }

            if ui.selectable_label(current_mode == "INFO+", "INFO+").clicked() {
                self.tail_state.log_level_filter.active = true;
                self.tail_state.log_level_filter.minimum_level = log_parser::LogLevel::Info;
            }

            if ui.selectable_label(current_mode == "WARN+", "WARN+").clicked() {
                self.tail_state.log_level_filter.active = true;
                self.tail_state.log_level_filter.minimum_level = log_parser::LogLevel::Warn;
            }

            if ui.selectable_label(current_mode == "ERROR", "ERROR").clicked() {
                self.tail_state.log_level_filter.active = true;
                self.tail_state.log_level_filter.minimum_level = log_parser::LogLevel::Error;
            }

            ui.separator();

            // Checkbox for showing unknown level lines
            if ui.checkbox(&mut self.tail_state.log_level_filter.show_unknown, "Show UNKNOWN")
                .on_hover_text("Show lines without detectable log level")
                .changed()
            {
                // Checkbox state updated automatically
            }
        });

        ui.separator();

        // Output area - use all available space
        let scroll_output = egui::ScrollArea::vertical()
            .id_salt("tail_output_scroll")
            .auto_shrink([false, false])
            .stick_to_bottom(self.tail_state.auto_scroll);

        scroll_output.show(ui, |ui| {
            // Add horizontal scrolling for long lines
            egui::ScrollArea::horizontal()
                .id_salt("tail_output_h_scroll")
                .show(ui, |ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                    
                    // Apply custom font size
                    let font_id = egui::FontId::new(self.tail_state.font_size, egui::FontFamily::Monospace);
                    ui.style_mut().text_styles.insert(egui::TextStyle::Monospace, font_id);

                    let is_filtered = self.tail_state.tree_filter.active && 
                                     self.tail_state.tree_filter.apply_to_output;
                    
                    for log_line in &self.tail_state.output_buffer {
                        // Check if this line should be visible based on tree filter
                        if is_filtered {
                            // Find the file that generated this log line
                            let should_show = self.tail_state.files.iter().any(|file| {
                                file.display_name == log_line.source_file &&
                                filter::tree::is_file_visible(
                                    &self.tail_state.tree_filter,
                                    &file.path.to_string_lossy(),
                                    &file.display_name
                                )
                            });

                            if !should_show {
                                continue;
                            }
                        }

                        // Check if this line should be visible based on log level filter
                        if !self.tail_state.log_level_filter.should_show_line(
                            &log_line.content,
                            &self.log_detector
                        ) {
                            continue;
                        }

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;

                            // Timestamp (relative)
                            let elapsed = log_line.timestamp.elapsed();
                            let secs = elapsed.as_secs();
                            let time_str = if secs < 60 {
                                format!("{}s", secs)
                            } else if secs < 3600 {
                                format!("{}m", secs / 60)
                            } else {
                                format!("{}h", secs / 3600)
                            };
                            ui.label(egui::RichText::new(time_str).color(egui::Color32::GRAY));

                            // Source file with color
                            let color = get_color_for_file(&log_line.source_file);
                            ui.colored_label(color, format!("[{}]", log_line.source_file));

                            // Content with log level coloring
                            let detected_level = self.log_detector.detect(&log_line.content);
                            let level_color = self.config.log_format.get_color_scheme().get_color(detected_level);
                            ui.colored_label(level_color, &log_line.content);
                        });
                    }

                    // Check if we're showing nothing due to filtering
                    let visible_count = self.tail_state.output_buffer.iter().filter(|log_line| {
                        // Check tree filter
                        if is_filtered {
                            let tree_visible = self.tail_state.files.iter().any(|file| {
                                file.display_name == log_line.source_file &&
                                filter::tree::is_file_visible(
                                    &self.tail_state.tree_filter,
                                    &file.path.to_string_lossy(),
                                    &file.display_name
                                )
                            });
                            if !tree_visible {
                                return false;
                            }
                        }

                        // Check log level filter
                        self.tail_state.log_level_filter.should_show_line(
                            &log_line.content,
                            &self.log_detector
                        )
                    }).count();
                    
                    if visible_count == 0 {
                        if is_filtered && !self.tail_state.output_buffer.is_empty() {
                            ui.label(
                                egui::RichText::new("No output from filtered files")
                                    .italics()
                                    .color(egui::Color32::from_rgb(255, 200, 100)),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new("Waiting for log output...")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                        }
                    }
                });
        });

        // Status bar
        ui.separator();
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.tail_state.auto_scroll, "Auto-scroll");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let buffer_pct = if self.tail_state.max_buffer_lines > 0 {
                    (self.tail_state.output_buffer.len() as f32
                        / self.tail_state.max_buffer_lines as f32)
                        * 100.0
                } else {
                    0.0
                };

                let active_count = self.tail_state.files.iter().filter(|f| f.is_active).count();

                ui.label(format!(
                    "Files: {}  Active: {}  Lines: {} / {}  Buffer: {:.1}%  Update: {}ms",
                    self.tail_state.files.len(),
                    active_count,
                    self.tail_state.output_buffer.len(),
                    self.tail_state.max_buffer_lines,
                    buffer_pct,
                    self.tail_state.poll_interval_ms
                ));

                if self.tail_state.lines_dropped > 0 {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!("  ⚠ Dropped: {}", self.tail_state.lines_dropped),
                    );
                }
            });
        });
    }

    pub fn render_tail_preview(&mut self, ui: &mut egui::Ui) {
        if let Some(file_idx) = self.tail_state.preview_selected_file {
            if file_idx < self.tail_state.files.len() {
                // Clone what we need before the closure
                let file_path = self.tail_state.files[file_idx].path.clone();
                let file_display_name = self.tail_state.files[file_idx].display_name.clone();
                let file_last_size = self.tail_state.files[file_idx].last_size;
                
                let mut open_editor = false;

                // Header
                ui.horizontal(|ui| {
                    // Extract parent directory for display
                    let parent_dir = file_path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    let header_text = if !parent_dir.is_empty() {
                        format!(
                            "Preview: {}/{} ({:.1} KB)",
                            parent_dir,
                            file_display_name,
                            file_last_size as f64 / 1024.0
                        )
                    } else {
                        format!(
                            "Preview: {} ({:.1} KB)",
                            file_display_name,
                            file_last_size as f64 / 1024.0
                        )
                    };
                    
                    // Label with tooltip showing full path
                    let label_response = ui.label(header_text);
                    label_response.on_hover_text(format!("Full path: {}", file_path.display()));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Pause/Follow toggle
                        let (icon, color) = match self.tail_state.preview_mode {
                            PreviewMode::Following => {
                                ("📍 Following", egui::Color32::from_rgb(100, 255, 100))
                            }
                            PreviewMode::Paused => {
                                ("⏸ Paused", egui::Color32::from_rgb(255, 200, 100))
                            }
                        };

                        if ui.button(egui::RichText::new(icon).color(color)).clicked() {
                            self.tail_state.preview_mode = match self.tail_state.preview_mode {
                                PreviewMode::Following => PreviewMode::Paused,
                                PreviewMode::Paused => PreviewMode::Following,
                            };
                        }
                        
                        ui.separator();
                        
                        // Open in Explorer button
                        if ui.button("📁 Explorer").on_hover_text("Open file location in Explorer/Finder").clicked() {
                            VisGrepApp::open_path_in_explorer(&file_path);
                        }
                        
                        // Open in Editor button
                        if ui.button("📝 Editor").on_hover_text("Open file in editor").clicked() {
                            open_editor = true;
                        }
                        
                        // Copy path button
                        if ui.button("📋 Copy Path").on_hover_text("Copy full file path to clipboard").clicked() {
                            use arboard::Clipboard;
                            match Clipboard::new() {
                                Ok(mut clipboard) => {
                                    let path_str = file_path.to_string_lossy().to_string();
                                    match clipboard.set_text(&path_str) {
                                        Ok(_) => log::info!("Copied path to clipboard: {}", path_str),
                                        Err(e) => log::error!("Failed to copy path: {}", e),
                                    }
                                }
                                Err(e) => log::error!("Failed to access clipboard: {}", e),
                            }
                        }
                        
                        ui.separator();
                        
                        // Buffer size control
                        ui.label("Lines:");
                        let response = ui.add(
                            egui::DragValue::new(&mut self.tail_state.preview_follow_lines)
                                .speed(50.0)
                                .range(100..=10000)
                                .prefix("📜 ")
                        );
                        
                        if response.changed() {
                            self.tail_state.preview_needs_reload = true;
                        }
                        
                        response.on_hover_text("Number of lines to keep in buffer (100-10000)");
                        
                        if ui.small_button("500").on_hover_text("500 lines").clicked() {
                            self.tail_state.preview_follow_lines = 500;
                            self.tail_state.preview_needs_reload = true;
                        }
                        if ui.small_button("1K").on_hover_text("1000 lines").clicked() {
                            self.tail_state.preview_follow_lines = 1000;
                            self.tail_state.preview_needs_reload = true;
                        }
                        if ui.small_button("5K").on_hover_text("5000 lines").clicked() {
                            self.tail_state.preview_follow_lines = 5000;
                            self.tail_state.preview_needs_reload = true;
                        }
                    });
                });

                ui.separator();

                // Filter input UI
                let mut scroll_to_match = false;
                if filter::preview::render_filter_input(ui, &mut self.tail_state.preview_filter) {
                    // Filter changed, update matches
                    scroll_to_match = filter::preview::update_filter_matches(
                        &mut self.tail_state.preview_filter,
                        &self.tail_state.preview_content
                    );
                }

                // Goto line input UI
                if self.tail_state.goto_line_active {
                    ui.horizontal(|ui| {
                        ui.label("Go to line:");

                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.tail_state.goto_line_input)
                                .desired_width(100.0)
                        );

                        // Auto-focus the input
                        response.request_focus();

                        // Check for Enter key press
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

                        // Handle Enter key or lost focus with Enter
                        if (response.lost_focus() && enter_pressed) || enter_pressed {
                            if let Ok(line_num) = self.tail_state.goto_line_input.parse::<usize>() {
                                if line_num > 0 && line_num <= self.tail_state.preview_content.len() {
                                    let target = line_num - 1; // Convert to 0-indexed
                                    info!("Goto line: user entered {}, setting target to {}", line_num, target);
                                    self.tail_state.goto_line_target = Some(target);
                                    self.tail_state.preview_mode = PreviewMode::Paused;
                                }
                            }
                            self.tail_state.goto_line_active = false;
                            self.tail_state.goto_line_input.clear();
                        }

                        // Show total lines
                        ui.label(format!("/ {}", self.tail_state.preview_content.len()));
                    });
                }

                // Check if we have a goto line target
                let goto_target = self.tail_state.goto_line_target;

                // Content area - use all available space
                let scroll_area = if self.tail_state.preview_mode == PreviewMode::Following {
                    egui::ScrollArea::both()
                        .stick_to_bottom(true)
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                } else {
                    egui::ScrollArea::both()
                        .scroll_offset(egui::Vec2::new(0.0, self.tail_state.preview_scroll_offset))
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                };

                let scroll_output = scroll_area
                    .id_salt("tail_preview_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                        // Apply custom font size
                        let font_id = egui::FontId::new(self.tail_state.font_size, egui::FontFamily::Monospace);
                        ui.style_mut().text_styles.insert(egui::TextStyle::Monospace, font_id);

                        // Display preview content
                        if self.tail_state.preview_content.is_empty() {
                            ui.label(
                                egui::RichText::new("Loading...")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                        } else {
                            let filter = &self.tail_state.preview_filter;

                            for (line_idx, line) in
                                self.tail_state.preview_content.iter().enumerate()
                            {
                                let is_match = filter.match_lines.contains(&line_idx);
                                let is_current = filter.current_match_line() == Some(line_idx);

                                // If we should scroll to this match, make it visible
                                if scroll_to_match && is_current {
                                    let line_height = self.tail_state.font_size + 4.0;
                                    let target_y = line_idx as f32 * line_height;
                                    ui.scroll_to_rect(
                                        egui::Rect::from_min_size(
                                            egui::pos2(0.0, target_y),
                                            egui::vec2(100.0, line_height)
                                        ),
                                        Some(egui::Align::Center)
                                    );
                                }

                                // If we should scroll to goto line target, make it visible
                                if let Some(target_line) = goto_target {
                                    if line_idx == target_line {
                                        info!("Scrolling to line_idx: {}, target_line: {}", line_idx, target_line);
                                        let line_height = self.tail_state.font_size + 4.0;
                                        let target_y = line_idx as f32 * line_height;
                                        ui.scroll_to_rect(
                                            egui::Rect::from_min_size(
                                                egui::pos2(0.0, target_y),
                                                egui::vec2(100.0, line_height)
                                            ),
                                            Some(egui::Align::Center)
                                        );
                                        // Clear the target after scrolling on next frame
                                        self.tail_state.goto_line_target = None;
                                    }
                                }

                                let color_scheme = self.config.log_format.get_color_scheme();
                                filter::preview::render_filtered_line(
                                    ui,
                                    line,
                                    line_idx + 1,
                                    is_match,
                                    is_current,
                                    filter,
                                    &self.log_detector,
                                    &color_scheme,
                                );
                            }
                        }
                    });

                // Detect manual scroll (switch to Paused mode)
                if self.tail_state.preview_mode == PreviewMode::Following {
                    // In Following mode, we don't track manual scrolls
                } else {
                    // Update scroll offset
                    self.tail_state.preview_scroll_offset = scroll_output.state.offset.y;
                }

                // Footer
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("j/k: scroll  gg/G: jump  /: filter  n/N: next/prev match")
                            .color(egui::Color32::GRAY)
                            .small(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.tail_state.preview_mode == PreviewMode::Following {
                            ui.label(
                                egui::RichText::new(format!(
                                    "> Following - showing last {} lines",
                                    self.tail_state.preview_follow_lines
                                ))
                                .color(egui::Color32::from_rgb(100, 255, 100)),
                            );
                        } else {
                            let total_lines = self.tail_state.preview_content.len();
                            ui.label(format!("Total lines: {}", total_lines));
                        }
                    });
                });
                
                // Handle editor opening outside of closures
                if open_editor {
                    self.open_file_in_editor(&file_path);
                }
            } else {
                // Invalid file index
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Error: Invalid file selection")
                            .italics()
                            .color(egui::Color32::RED),
                    );
                });
            }
        } else {
            // No file selected
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("← Select a file to preview")
                        .italics()
                        .color(egui::Color32::GRAY),
                );
            });
        }
    }

    pub fn handle_tail_mode_navigation(&mut self, ctx: &egui::Context) {
        // Handle global tail mode shortcuts
        ctx.input(|i| {
            // + or = - increase update rate (decrease interval)
            if i.key_pressed(egui::Key::Plus) || 
               (i.key_pressed(egui::Key::Equals) && !i.modifiers.shift) {
                self.tail_state.poll_interval_ms = match self.tail_state.poll_interval_ms {
                    ms if ms > 1000 => 1000,
                    ms if ms > 500 => 500,
                    ms if ms > 250 => 250,
                    ms if ms > 100 => 100,
                    _ => 50, // Minimum 50ms (20 updates/sec)
                };
            }
            
            // - - decrease update rate (increase interval)
            if i.key_pressed(egui::Key::Minus) {
                self.tail_state.poll_interval_ms = match self.tail_state.poll_interval_ms {
                    ms if ms < 100 => 100,
                    ms if ms < 250 => 250,
                    ms if ms < 500 => 500,
                    ms if ms < 1000 => 1000,
                    ms if ms < 2000 => 2000,
                    _ => 5000, // Maximum 5000ms (0.2 updates/sec)
                };
            }

            // L - cycle log level filter (ALL -> INFO+ -> WARN+ -> ERROR -> ALL)
            if i.key_pressed(egui::Key::L) && !i.modifiers.shift {
                self.tail_state.log_level_filter.cycle_mode();
            }

            // Shift+L - cycle log level filter backwards (ALL -> ERROR -> WARN+ -> INFO+ -> ALL)
            if i.key_pressed(egui::Key::L) && i.modifiers.shift {
                self.tail_state.log_level_filter.cycle_mode_backwards();
            }
        });
        
        // Handle preview navigation (if a file is selected)
        if self.tail_state.preview_selected_file.is_some() {
            ctx.input(|i| {
                // / - activate filter
                if i.key_pressed(egui::Key::Slash) && !self.tail_state.preview_filter.active {
                    self.tail_state.preview_filter.activate();
                }
                
                // Escape - deactivate filter or goto line mode
                if i.key_pressed(egui::Key::Escape) {
                    if self.tail_state.preview_filter.active {
                        self.tail_state.preview_filter.deactivate();
                    } else if self.tail_state.goto_line_active {
                        self.tail_state.goto_line_active = false;
                        self.tail_state.goto_line_input.clear();
                        self.tail_state.goto_line_target = None;
                    }
                }

                // : - activate goto line mode
                if !self.tail_state.preview_filter.active && !self.tail_state.goto_line_active {
                    if i.events.iter().any(|e| matches!(e, egui::Event::Text(s) if s == ":")) {
                        self.tail_state.goto_line_active = true;
                        self.tail_state.goto_line_input.clear();
                    }
                }

                // n - next match
                if i.key_pressed(egui::Key::N) && !i.modifiers.shift && self.tail_state.preview_filter.active {
                    self.tail_state.preview_filter.next_match();
                    if let Some(line_idx) = self.tail_state.preview_filter.current_match_line() {
                        // Calculate scroll position to center the match
                        let line_height = 20.0; // Approximate line height
                        self.tail_state.preview_scroll_offset = (line_idx as f32 * line_height).max(0.0);
                        self.tail_state.preview_mode = PreviewMode::Paused;
                    }
                }
                
                // N (Shift+n) - previous match  
                if i.key_pressed(egui::Key::N) && i.modifiers.shift && self.tail_state.preview_filter.active {
                    self.tail_state.preview_filter.prev_match();
                    if let Some(line_idx) = self.tail_state.preview_filter.current_match_line() {
                        // Calculate scroll position to center the match
                        let line_height = 20.0; // Approximate line height
                        self.tail_state.preview_scroll_offset = (line_idx as f32 * line_height).max(0.0);
                        self.tail_state.preview_mode = PreviewMode::Paused;
                    }
                }
                
                // j - scroll down
                if i.key_pressed(egui::Key::J) && !i.modifiers.ctrl {
                    self.tail_state.preview_scroll_offset += 20.0;
                    self.tail_state.preview_mode = PreviewMode::Paused;
                }
                // k - scroll up
                if i.key_pressed(egui::Key::K) && !i.modifiers.ctrl {
                    self.tail_state.preview_scroll_offset =
                        (self.tail_state.preview_scroll_offset - 20.0).max(0.0);
                    self.tail_state.preview_mode = PreviewMode::Paused;
                }
                // g - handle gg (jump to top) or G (jump to bottom and follow)
                if i.key_pressed(egui::Key::G) {
                    if i.modifiers.shift {
                        // Shift+G - jump to end and resume following
                        self.tail_state.preview_mode = PreviewMode::Following;
                        self.tail_state.preview_scroll_offset = 0.0;
                    } else {
                        // g (will be gg with double-tap, but for now just jump to top)
                        self.tail_state.preview_scroll_offset = 0.0;
                        self.tail_state.preview_mode = PreviewMode::Paused;
                    }
                }
                // Ctrl+D - page down
                if i.key_pressed(egui::Key::D) && i.modifiers.ctrl {
                    self.tail_state.preview_scroll_offset += 400.0;
                    self.tail_state.preview_mode = PreviewMode::Paused;
                }
                // Ctrl+U - page up
                if i.key_pressed(egui::Key::U) && i.modifiers.ctrl {
                    self.tail_state.preview_scroll_offset =
                        (self.tail_state.preview_scroll_offset - 400.0).max(0.0);
                    self.tail_state.preview_mode = PreviewMode::Paused;
                }
            });
        }
    }
    
    // This is called from main.rs but kept for compatibility
    pub fn render_tail_mode_ui(&mut self, ui: &mut egui::Ui) {
        self.render_tail_mode_controls(ui);
    }
}