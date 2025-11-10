use crate::VisGrepApp;
use eframe::egui;

impl VisGrepApp {
    pub fn render_grep_mode_ui(&mut self, ui: &mut egui::Ui) {
        // Search controls
        self.render_highlight_pattern_field(ui);
        ui.separator();

        self.render_search_path_field(ui);
        ui.separator();

        self.render_search_query_field(ui);
        ui.separator();

        // File age filter
        self.render_file_age_filter(ui);
        ui.separator();

        // Results filter and expand/collapse controls
        ui.horizontal(|ui| {
            ui.label("Filter Results:");
            ui.add(
                egui::TextEdit::singleline(&mut self.grep_state.results_filter)
                    .desired_width(300.0),
            );
            if ui.small_button("Clear").clicked() {
                self.grep_state.results_filter.clear();
            }

            ui.separator();

            if ui.button("Expand All").clicked() {
                for i in 0..self.grep_state.results.len() {
                    self.grep_state.collapsing_state.insert(i, true);
                }
            }
            if ui.button("Collapse All").clicked() {
                for i in 0..self.grep_state.results.len() {
                    self.grep_state.collapsing_state.insert(i, false);
                }
            }
        });
        ui.separator();

        // Main content area - the panels will be handled in the main update loop
        // for proper splitter functionality
    }
    
    pub fn render_grep_left_panel(&mut self, ui: &mut egui::Ui) {
        // Results
        let available_height = ui.available_height();
        
        egui::ScrollArea::vertical()
            .id_salt("results_scroll")
            .max_height(available_height * 0.4)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if self.grep_state.searching {
                    ui.label("Searching...");
                } else if self.grep_state.results.is_empty()
                    && !self.grep_state.search_query.is_empty()
                {
                    ui.label("No results found");
                } else {
                    self.render_results(ui);
                }
            });

        ui.separator();

        // Matched Line Focus Panel
        ui.label("Matched Line:");
        egui::Frame::none()
            .fill(egui::Color32::from_rgb(40, 40, 50))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                self.render_matched_line_focus(ui);
            });
    }
    
    pub fn render_grep_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.label("Preview:");
        
        let remaining_height = ui.available_height();

        // Add horizontal scrolling to handle long lines
        let scroll_area = egui::ScrollArea::both()
            .id_salt("preview_scroll")
            .max_height(remaining_height)
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);

        // Only force scroll position when a new match is selected
        let scroll_area = if self.should_scroll_to_match {
            self.should_scroll_to_match = false;
            scroll_area.scroll_offset(egui::Vec2::new(0.0, self.preview_scroll_offset))
        } else {
            scroll_area
        };

        scroll_area.show(ui, |ui| {
            self.render_preview(ui);
        });
    }

    pub fn handle_grep_mode_background_tasks(&mut self) {
        // Debounced search handling
        if self.grep_state.pending_search
            && self.grep_state.last_search_time.elapsed()
                > std::time::Duration::from_millis(500)
            && !self.grep_state.search_query.is_empty()
        {
            self.perform_search();
        }
    }
}