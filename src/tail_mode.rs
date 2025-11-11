use crate::{PreviewMode, VisGrepApp, get_color_for_file};
use eframe::egui;

impl VisGrepApp {
    pub fn render_tail_mode_controls(&mut self, ui: &mut egui::Ui) {
        // Debug info at top
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("DEBUG: Panel height={:.1}, Available={:.1}", 
                    self.tail_state.control_panel_height, 
                    ui.available_height()))
                    .small()
                    .color(egui::Color32::YELLOW)
            );
        });
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
                        if !has_ungrouped {
                            has_ungrouped = true;
                            ui.separator();
                            ui.label(egui::RichText::new("Ungrouped Files").strong());
                        }
                        self.render_file_entry(ui, idx, 0);
                    }
                }
            });
        } else {
            // Flat list mode (original)
            ui.vertical(|ui| {
                for idx in 0..self.tail_state.files.len() {
                    self.render_file_entry(ui, idx, 0);
                }
            });
        }
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
            let indent = depth as f32 * 20.0;
            
            ui.horizontal(|ui| {
                ui.add_space(indent);
                
                // Expand/collapse arrow
                let arrow = if collapsed { "▶" } else { "▼" };
                if ui.small_button(arrow).clicked() {
                    // Toggle collapsed state
                    if let Some(layout) = &mut self.tail_state.layout {
                        if let Some(group) = layout.find_group_mut(group_id) {
                            group.collapsed = !group.collapsed;
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
        let indent = depth as f32 * 20.0;
        
        ui.horizontal(|ui| {
            ui.add_space(indent);
            
            // Activity indicator
            let indicator = if file.is_active { "●" } else { "○" };
            let color = if file.is_active {
                egui::Color32::from_rgb(0, 255, 0)
            } else {
                egui::Color32::GRAY
            };
            ui.colored_label(color, indicator);

            // Filename (selectable) with fixed width
            let selected = self.tail_state.preview_selected_file == Some(file_idx);
            ui.allocate_ui_with_layout(
                egui::Vec2::new(300.0, 20.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    if ui.selectable_label(selected, &file.display_name).clicked() {
                        self.tail_state.preview_selected_file = Some(file_idx);
                        self.tail_state.preview_needs_reload = true;
                        self.tail_state.preview_mode = PreviewMode::Following;
                    }
                },
            );

            // File size
            ui.label(format!("{:.1} KB", file.last_size as f64 / 1024.0));

            // Activity info
            if file.is_active && file.lines_since_last_read > 0 {
                ui.label(
                    egui::RichText::new(format!("(+{} lines)", file.lines_since_last_read))
                        .color(egui::Color32::from_rgb(255, 200, 100))
                );
            } else if !file.is_active {
                ui.label("(idle)");
            } else {
                ui.add_space(50.0);
            }

            // Pause button
            if ui.small_button(if file.paused { "▶" } else { "⏸" }).clicked() {
                file.paused = !file.paused;
            }
        });
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
            ui.label("Output (Combined):");

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

                    for log_line in &self.tail_state.output_buffer {
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

                            // Content
                            ui.label(&log_line.content);
                        });
                    }

                    if self.tail_state.output_buffer.is_empty() {
                        ui.label(
                            egui::RichText::new("Waiting for log output...")
                                .italics()
                                .color(egui::Color32::GRAY),
                        );
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
                let file = &self.tail_state.files[file_idx];

                // Header
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Preview: {} ({:.1} KB)",
                        file.display_name,
                        file.last_size as f64 / 1024.0
                    ));

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
                    });
                });

                ui.separator();

                // Content area
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
                            for (line_num, line) in
                                self.tail_state.preview_content.iter().enumerate()
                            {
                                ui.horizontal(|ui| {
                                    // Line number
                                    ui.label(
                                        egui::RichText::new(format!("{:4} ", line_num + 1))
                                            .color(egui::Color32::GRAY),
                                    );
                                    // Content
                                    ui.label(line);
                                });
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
                        egui::RichText::new("j/k: scroll  gg/G: jump")
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
        });
        
        // Handle preview navigation (if a file is selected)
        if self.tail_state.preview_selected_file.is_some() {
            ctx.input(|i| {
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