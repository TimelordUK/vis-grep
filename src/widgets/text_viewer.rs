use eframe::egui;
use log::info;
use std::collections::HashMap;
use crate::filter;
use crate::log_parser::{LogLevelDetector, LogColorScheme};
use crate::input_handler::{InputHandler, NavigationCommand};

/// View mode determines scrolling behavior
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    /// Auto-scroll to bottom as new content arrives
    Following,
    /// Manual navigation, scroll position is preserved
    Paused,
}

/// State for the text viewer widget
pub struct TextViewerState {
    /// Current view mode (following/paused)
    pub view_mode: ViewMode,

    /// Scroll offset when in Paused mode
    pub scroll_offset: f32,

    /// Filter state
    pub filter: filter::PreviewFilter,

    /// Font size for rendering
    pub font_size: f32,

    /// Goto line mode active
    pub goto_line_active: bool,

    /// Goto line input buffer
    pub goto_line_input: String,

    /// Target line to scroll to (0-indexed)
    pub goto_line_target: Option<usize>,

    /// Flag to scroll to bottom on next frame
    pub scroll_to_bottom: bool,

    /// Flag to scroll to current filter match on next frame
    pub scroll_to_current_match: bool,

    /// Bookmarks/marks (vim-style ma, 'a) - maps mark character to line number (0-indexed)
    pub marks: HashMap<char, usize>,

    /// Track the last line we explicitly navigated to (for mark setting)
    pub last_navigated_line: Option<usize>,

    /// Input handler for vim-style navigation
    pub input_handler: InputHandler,
}

impl TextViewerState {
    pub fn new(font_size: f32) -> Self {
        Self {
            view_mode: ViewMode::Following,
            scroll_offset: 0.0,
            filter: filter::PreviewFilter::new(),
            font_size,
            goto_line_active: false,
            goto_line_input: String::new(),
            goto_line_target: None,
            scroll_to_bottom: false,
            scroll_to_current_match: false,
            marks: HashMap::new(),
            last_navigated_line: None,
            input_handler: InputHandler::new(),
        }
    }
}

/// Reusable text viewer widget with vim-style navigation
pub struct TextViewer<'a> {
    state: &'a mut TextViewerState,
    content: &'a [String],
    log_detector: &'a LogLevelDetector,
    color_scheme: &'a LogColorScheme,
}

impl<'a> TextViewer<'a> {
    pub fn new(
        state: &'a mut TextViewerState,
        content: &'a [String],
        log_detector: &'a LogLevelDetector,
        color_scheme: &'a LogColorScheme,
    ) -> Self {
        Self {
            state,
            content,
            log_detector,
            color_scheme,
        }
    }

    /// Render the text viewer widget
    pub fn show(mut self, ui: &mut egui::Ui) {
        // Handle filter input and update matches if filter changed
        let mut scroll_to_match = false;
        if filter::preview::render_filter_input(ui, &mut self.state.filter) {
            // Filter changed, update matches
            scroll_to_match = filter::preview::update_filter_matches(
                &mut self.state.filter,
                self.content
            );
        }

        // Check if we should scroll to current match (from n/N navigation)
        if self.state.scroll_to_current_match {
            scroll_to_match = true;
            self.state.scroll_to_current_match = false;
        }

        // Handle goto line input
        self.render_goto_line_input(ui);

        // Capture goto target and scroll_to_bottom flag for use inside scroll area
        let goto_target = self.state.goto_line_target;
        let scroll_to_bottom = self.state.scroll_to_bottom;

        // Content area - use all available space
        // When we have a goto_line_target or scroll_to_bottom, don't set scroll_offset - let scroll_to_rect handle it
        let scroll_area = if self.state.view_mode == ViewMode::Following {
            egui::ScrollArea::both()
                .stick_to_bottom(true)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        } else if goto_target.is_some() || scroll_to_bottom {
            // Don't set scroll_offset when goto or scroll_to_bottom is active
            egui::ScrollArea::both()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        } else {
            egui::ScrollArea::both()
                .scroll_offset(egui::Vec2::new(0.0, self.state.scroll_offset))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        };

        let scroll_output = scroll_area
            .id_salt("text_viewer_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                // Apply custom font size
                let font_id = egui::FontId::new(self.state.font_size, egui::FontFamily::Monospace);
                ui.style_mut().text_styles.insert(egui::TextStyle::Monospace, font_id);

                if self.content.is_empty() {
                    ui.label(
                        egui::RichText::new("No content to display")
                            .italics()
                            .color(egui::Color32::GRAY),
                    );
                } else {
                    for (line_idx, line) in self.content.iter().enumerate() {
                        let is_match = self.state.filter.match_lines.contains(&line_idx);
                        let is_current = self.state.filter.current_match_line() == Some(line_idx);
                        let is_last_line = line_idx == self.content.len() - 1;

                        let response = filter::preview::render_filtered_line(
                            ui,
                            line,
                            line_idx + 1,
                            is_match,
                            is_current,
                            &self.state.filter,
                            self.log_detector,
                            self.color_scheme,
                        );

                        // If we should scroll to this match, make it visible using actual rect
                        if scroll_to_match && is_current {
                            ui.scroll_to_rect(response.rect, Some(egui::Align::Center));
                        }

                        // If we should scroll to goto line target, make it visible using actual rect
                        if let Some(target_line) = goto_target {
                            if line_idx == target_line {
                                info!("Scrolling to line_idx: {}, target_line: {}, rect: {:?}", line_idx, target_line, response.rect);
                                ui.scroll_to_rect(response.rect, Some(egui::Align::Center));
                            }
                        }

                        // If we should scroll to bottom (G command), scroll to last line
                        if scroll_to_bottom && is_last_line {
                            info!("Scrolling to bottom (last line): {}", line_idx);
                            // Scroll to show last line at bottom of viewport
                            ui.scroll_to_rect(response.rect, None);
                        }
                    }
                }
            });

        // Clear goto target and scroll_to_bottom after scroll area completes
        if goto_target.is_some() {
            info!("Clearing goto_line_target after scroll area");
            self.state.goto_line_target = None;
        }
        if scroll_to_bottom {
            info!("Clearing scroll_to_bottom flag after scroll area");
            self.state.scroll_to_bottom = false;
        }

        // Update scroll offset
        if self.state.view_mode == ViewMode::Following {
            // In Following mode, we don't track manual scrolls
        } else {
            if goto_target.is_none() && !scroll_to_bottom {
                // Normal scrolling - just update offset
                self.state.scroll_offset = scroll_output.state.offset.y;
            } else {
                // After goto or scroll_to_bottom, save the new offset
                info!("After goto/scroll_to_bottom, scroll offset is now: {}", scroll_output.state.offset.y);
                self.state.scroll_offset = scroll_output.state.offset.y;
            }
        }

        // Footer with controls hint
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("j/k: scroll  gg/G: jump  /: filter  n/N: next/prev match  :: goto line  ma/'a: mark/goto")
                    .color(egui::Color32::GRAY)
                    .small(),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.state.view_mode == ViewMode::Following {
                    ui.label(
                        egui::RichText::new(format!("FOLLOWING ({} lines)", self.content.len()))
                            .color(egui::Color32::GREEN)
                            .small(),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(format!("PAUSED ({} lines)", self.content.len()))
                            .color(egui::Color32::YELLOW)
                            .small(),
                    );
                }
            });
        });
    }

    fn render_goto_line_input(&mut self, ui: &mut egui::Ui) {
        if self.state.goto_line_active {
            ui.horizontal(|ui| {
                ui.label(":");

                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.state.goto_line_input)
                        .desired_width(100.0)
                        .hint_text("line number")
                );

                // Auto-focus when activated
                response.request_focus();

                // Check for Enter key press
                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

                // Handle Enter key or lost focus with Enter
                if (response.lost_focus() && enter_pressed) || enter_pressed {
                    if let Ok(line_num) = self.state.goto_line_input.parse::<usize>() {
                        if line_num > 0 && line_num <= self.content.len() {
                            let target = line_num - 1; // Convert to 0-indexed
                            info!("Goto line: user entered {}, setting target to {}", line_num, target);
                            self.state.goto_line_target = Some(target);
                            self.state.last_navigated_line = Some(target);
                            self.state.view_mode = ViewMode::Paused;
                        }
                    }
                    self.state.goto_line_active = false;
                    self.state.goto_line_input.clear();
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.state.goto_line_active = false;
                    self.state.goto_line_input.clear();
                }
            });
        }
    }

    /// Handle keyboard input for vim-style navigation
    /// Call this from your event handler to process navigation commands
    pub fn handle_input(
        state: &mut TextViewerState,
        _content: &[String],
        ctx: &egui::Context,
    ) -> bool {
        // Check if any text input is focused (skip vim keys if typing)
        if ctx.wants_keyboard_input() {
            return false;
        }

        let mut handled = false;

        // Handle goto line and filter activation (not in InputHandler)
        ctx.input(|i| {
            // Handle goto line command
            if i.key_pressed(egui::Key::Colon) && !state.goto_line_active && !state.filter.active {
                state.goto_line_active = true;
                state.goto_line_input.clear();
                handled = true;
                return;
            }

            // Handle filter activation
            if i.key_pressed(egui::Key::Slash) && !state.filter.active && !state.goto_line_active {
                state.filter.activate();
                handled = true;
                return;
            }

            // Filter navigation (n/N when filter is active)
            if state.filter.active {
                if i.key_pressed(egui::Key::N) {
                    if i.modifiers.shift {
                        state.filter.prev_match();
                    } else {
                        state.filter.next_match();
                    }
                    state.scroll_to_current_match = true;
                    handled = true;
                    return;
                }
            }

            // Simple j/k scrolling (not in InputHandler, too specific)
            if !state.goto_line_active && !state.filter.active {
                if i.key_pressed(egui::Key::J) {
                    state.scroll_offset += state.font_size + 4.0;
                    state.view_mode = ViewMode::Paused;
                    handled = true;
                } else if i.key_pressed(egui::Key::K) {
                    state.scroll_offset = (state.scroll_offset - (state.font_size + 4.0)).max(0.0);
                    state.view_mode = ViewMode::Paused;
                    handled = true;
                }
            }
        });

        // Use InputHandler for gg/G and other complex navigation
        if !state.goto_line_active && !state.filter.active && !handled {
            if let Some(command) = state.input_handler.process_input(ctx) {
                match command {
                    NavigationCommand::FirstMatch => {
                        // gg - go to top
                        state.scroll_offset = 0.0;
                        state.view_mode = ViewMode::Paused;
                        handled = true;
                    }
                    NavigationCommand::LastMatch => {
                        // G - scroll to bottom (flag for next frame)
                        state.scroll_to_bottom = true;
                        state.view_mode = ViewMode::Paused;
                        handled = true;
                    }
                    NavigationCommand::NextMatch => {
                        // n - next match (when filter active, handled above)
                        if state.filter.active {
                            state.filter.next_match();
                            handled = true;
                        }
                    }
                    NavigationCommand::PreviousMatch => {
                        // p - previous match (when filter active)
                        if state.filter.active {
                            state.filter.prev_match();
                            handled = true;
                        }
                    }
                    NavigationCommand::SetMark(mark_char) => {
                        // ma, mb, etc - set a mark at current line
                        // Prefer last_navigated_line if set (from :goto or 'mark navigation)
                        // Otherwise estimate from scroll_offset
                        let mark_line = if let Some(line) = state.last_navigated_line {
                            line
                        } else {
                            let line_height = state.font_size + 4.0;
                            (state.scroll_offset / line_height) as usize
                        };
                        state.marks.insert(mark_char, mark_line);
                        info!("Set mark '{}' at line {} (1-indexed: {})", mark_char, mark_line, mark_line + 1);
                        handled = true;
                    }
                    NavigationCommand::GotoMark(mark_char) => {
                        // 'a, 'b, etc - go to a mark
                        if let Some(&line_num) = state.marks.get(&mark_char) {
                            info!("Going to mark '{}' at line {}", mark_char, line_num);
                            state.goto_line_target = Some(line_num);
                            state.last_navigated_line = Some(line_num);
                            state.view_mode = ViewMode::Paused;
                            handled = true;
                        } else {
                            info!("Mark '{}' not set", mark_char);
                        }
                    }
                    _ => {
                        // Other commands not applicable to text viewer
                    }
                }
            }
        }

        handled
    }
}
