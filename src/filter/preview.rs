use eframe::egui::{self, Color32, TextEdit, RichText, TextStyle};
use super::state::PreviewFilter;
use crate::log_parser::{LogLevelDetector, LogColorScheme};

pub fn render_filter_input(ui: &mut egui::Ui, filter: &mut PreviewFilter) -> bool {
    let mut filter_changed = false;

    if filter.active {
        ui.horizontal(|ui| {
            ui.label("Filter:");

            let text_edit = TextEdit::singleline(&mut filter.query)
                .desired_width(200.0)
                .font(TextStyle::Monospace);

            let response = ui.add(text_edit);

            // Request focus if filter was just activated
            if filter.request_focus {
                response.request_focus();
                filter.request_focus = false;
            }

            if response.changed() {
                filter.update_query(filter.query.clone());
                filter_changed = true;
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                filter.deactivate();
            }

            // Show match statistics
            if !filter.match_lines.is_empty() {
                let (current, total) = filter.match_stats();
                ui.label(format!("{} of {} matches", current, total));
            } else if !filter.query.is_empty() {
                ui.label("No matches");
            }

            // Show filter mode
            if filter.use_regex {
                ui.label(RichText::new("regex").color(Color32::from_rgb(100, 150, 255)));
            } else if filter.case_sensitive {
                ui.label(RichText::new("case").color(Color32::from_rgb(100, 150, 255)));
            }
        });
    }

    filter_changed
}

pub fn render_filtered_line(
    ui: &mut egui::Ui,
    line: &str,
    line_number: usize,
    is_match: bool,
    is_current_match: bool,
    filter: &PreviewFilter,
    log_detector: &LogLevelDetector,
    color_scheme: &LogColorScheme,
) -> egui::Response {
    let bg_color = if is_current_match {
        Color32::from_rgb(80, 80, 0)  // Yellow highlight for current match
    } else if is_match {
        Color32::from_rgb(40, 40, 80)  // Blue highlight for matches
    } else {
        Color32::TRANSPARENT
    };

    ui.horizontal(|ui| {
        if bg_color != Color32::TRANSPARENT {
            ui.painter().rect_filled(
                ui.available_rect_before_wrap(),
                0.0,
                bg_color,
            );
        }

        // Line number - painted directly so it's not selectable
        let line_num_text = format!("{:>4} ", line_number);
        let font_id = egui::FontId::monospace(ui.text_style_height(&egui::TextStyle::Body));
        let galley = ui.painter().layout_no_wrap(
            line_num_text,
            font_id,
            Color32::from_gray(128),
        );

        let line_num_pos = ui.cursor().min;
        ui.painter().galley(line_num_pos, galley.clone(), Color32::from_gray(128));

        // Allocate space for the line number
        ui.allocate_space(galley.size());

        // Line content with match highlighting and log level coloring (selectable)
        let log_level = log_detector.detect(line);
        let base_color = color_scheme.get_color(log_level);

        if is_match && filter.active {
            render_highlighted_text(ui, line, filter, base_color);
        } else {
            ui.label(RichText::new(line).monospace().color(base_color));
        }
    }).response
}

fn render_highlighted_text(ui: &mut egui::Ui, text: &str, filter: &PreviewFilter, base_color: Color32) {
    let matches = filter.find_matches(text);

    if matches.is_empty() {
        ui.label(RichText::new(text).monospace().color(base_color));
        return;
    }

    let mut last_end = 0;

    ui.horizontal_wrapped(|ui| {
        for (start, end) in matches {
            // Text before match
            if start > last_end {
                ui.label(RichText::new(&text[last_end..start]).monospace().color(base_color));
            }

            // Highlighted match
            ui.label(
                RichText::new(&text[start..end])
                    .monospace()
                    .background_color(Color32::from_rgb(255, 255, 0))
                    .color(Color32::BLACK)
            );

            last_end = end;
        }

        // Remaining text after last match
        if last_end < text.len() {
            ui.label(RichText::new(&text[last_end..]).monospace().color(base_color));
        }
    });
}

pub fn handle_filter_navigation(filter: &mut PreviewFilter, key: egui::Key, shift_pressed: bool) -> Option<usize> {
    match key {
        egui::Key::N if !shift_pressed => {
            filter.next_match();
            filter.current_match_line()
        }
        egui::Key::N if shift_pressed => {
            filter.prev_match();
            filter.current_match_line()
        }
        _ => None,
    }
}

pub fn update_filter_matches(filter: &mut PreviewFilter, lines: &[String]) -> bool {
    filter.match_lines.clear();

    if !filter.active || filter.query.is_empty() {
        filter.current_match = None;
        return false;
    }

    for (idx, line) in lines.iter().enumerate() {
        if filter.matches_line(line) {
            filter.match_lines.push(idx);
        }
    }

    // Set to first match if we have matches, or None if no matches
    filter.current_match = if filter.match_lines.is_empty() {
        None
    } else {
        Some(0)
    };

    // Return true if we have matches (should scroll to first)
    !filter.match_lines.is_empty()
}