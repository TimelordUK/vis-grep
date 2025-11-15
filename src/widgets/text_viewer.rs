use eframe::egui;
use log::info;
use crate::filter;
use crate::log_parser::{LogLevelDetector, LogColorScheme};

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
        // Handle filter input
        filter::preview::render_filter_input(ui, &mut self.state.filter);

        // Handle goto line input
        self.render_goto_line_input(ui);

        // Determine if we should scroll to a search match
        let scroll_to_match = if let Some(_) = self.state.filter.current_match {
            true
        } else {
            false
        };

        // Capture goto target for use inside scroll area
        let goto_target = self.state.goto_line_target;

        // Content area - use all available space
        // When we have a goto_line_target, don't set scroll_offset - let scroll_to_rect handle it
        let scroll_area = if self.state.view_mode == ViewMode::Following {
            egui::ScrollArea::both()
                .stick_to_bottom(true)
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        } else if goto_target.is_some() {
            // Don't set scroll_offset when goto is active - let scroll_to_rect work
            egui::ScrollArea::both()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        } else {
            egui::ScrollArea::both()
                .scroll_offset(egui::Vec2::new(0.0, self.state.scroll_offset))
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        };

        let scroll_output = scroll_area.show(ui, |ui| {
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
                }
            }
        });

        // Clear goto target after scroll area completes
        if goto_target.is_some() {
            info!("Clearing goto_line_target after scroll area");
            self.state.goto_line_target = None;
        }

        // Update scroll offset
        if self.state.view_mode == ViewMode::Following {
            // In Following mode, we don't track manual scrolls
        } else {
            if goto_target.is_none() {
                self.state.scroll_offset = scroll_output.state.offset.y;
            } else {
                info!("After goto, scroll offset is now: {}", scroll_output.state.offset.y);
                // Save the new offset for next frame
                self.state.scroll_offset = scroll_output.state.offset.y;
            }
        }

        // Footer with controls hint
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("j/k: scroll  gg/G: jump  /: filter  n/N: next/prev match  :: goto line")
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
    /// Returns true if the input was handled
    pub fn handle_input(
        state: &mut TextViewerState,
        content: &[String],
        ctx: &egui::Context,
    ) -> bool {
        use crate::input_handler::InputHandler;

        // Check if any text input is focused (skip vim keys if typing)
        if ctx.wants_keyboard_input() {
            return false;
        }

        let mut handled = false;

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

            // Filter navigation
            if state.filter.active {
                if i.key_pressed(egui::Key::N) {
                    if i.modifiers.shift {
                        state.filter.prev_match();
                    } else {
                        state.filter.next_match();
                    }
                    handled = true;
                    return;
                }
            }

            // Vim navigation (only when not in command/filter mode)
            if !state.goto_line_active && !state.filter.active {
                // j/k scrolling
                if i.key_pressed(egui::Key::J) {
                    state.scroll_offset += state.font_size + 4.0;
                    state.view_mode = ViewMode::Paused;
                    handled = true;
                } else if i.key_pressed(egui::Key::K) {
                    state.scroll_offset = (state.scroll_offset - (state.font_size + 4.0)).max(0.0);
                    state.view_mode = ViewMode::Paused;
                    handled = true;
                }

                // gg/G navigation
                if i.key_pressed(egui::Key::G) {
                    if i.modifiers.shift {
                        // G - go to end
                        state.scroll_offset = f32::MAX;
                        state.view_mode = ViewMode::Paused;
                    } else {
                        // gg - go to top (need to detect double-g somehow)
                        state.scroll_offset = 0.0;
                        state.view_mode = ViewMode::Paused;
                    }
                    handled = true;
                }
            }
        });

        handled
    }
}
