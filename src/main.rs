use arboard::Clipboard;
use eframe::egui;
use log::info;
use std::collections::HashMap;
use std::time::Instant;

mod config;
mod input_handler;
mod preview;
mod search;

use config::Config;
use input_handler::{InputHandler, NavigationCommand};
use preview::FilePreview;
use search::{SearchEngine, SearchResult};

struct VisGrepApp {
    search_path: String,
    file_pattern: String,
    search_query: String,
    case_sensitive: bool,
    use_regex: bool,
    recursive: bool,
    file_age_hours: Option<u64>,

    search_engine: SearchEngine,
    results: Vec<SearchResult>,
    selected_result: Option<usize>,
    preview: FilePreview,

    searching: bool,

    // UI state
    results_filter: String,
    collapsing_state: HashMap<usize, bool>,
    last_search_time: Instant,
    pending_search: bool,
    preview_scroll_offset: f32,
    should_scroll_to_match: bool, // Only scroll when a new match is selected
    scroll_to_selected_result: bool, // Flag to scroll results panel to selected item

    input_handler: InputHandler,
    marks: HashMap<char, usize>, // Store marks (a-z) -> result_id

    // FIX message highlighting pattern
    fix_highlight_pattern: String,

    // Configuration
    config: Config,
}

impl Default for VisGrepApp {
    fn default() -> Self {
        Self {
            search_path: Self::expand_tilde(
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .as_ref(),
            ),
            file_pattern: String::from("*.log"),
            search_query: String::new(),
            case_sensitive: false,
            use_regex: true,
            recursive: true,
            file_age_hours: None,

            search_engine: SearchEngine::new(),
            results: Vec::new(),
            selected_result: None,
            preview: FilePreview::new(),

            searching: false,

            results_filter: String::new(),
            collapsing_state: HashMap::new(),
            last_search_time: Instant::now(),
            pending_search: false,
            preview_scroll_offset: 0.0,
            should_scroll_to_match: false,
            scroll_to_selected_result: false,

            input_handler: InputHandler::new(),
            marks: HashMap::new(),

            fix_highlight_pattern: String::new(),

            config: Config::load(),
        }
    }
}

impl VisGrepApp {
    /// Expand ~ to home directory
    fn expand_tilde(path: &str) -> String {
        if let Some(stripped) = path.strip_prefix("~/") {
            if let Some(home) = std::env::var_os("HOME") {
                return format!("{}/{}", home.to_string_lossy(), stripped);
            }
        }
        path.to_string()
    }

    fn perform_search(&mut self) {
        // Expand tilde in search path
        let expanded_path = Self::expand_tilde(&self.search_path);

        info!(
            "Starting search: path='{}', pattern='{}', query='{}', file_age={:?}hrs",
            &expanded_path, &self.file_pattern, &self.search_query, &self.file_age_hours
        );
        self.searching = true;
        self.pending_search = false;
        let start = Instant::now();
        self.results = self.search_engine.search(
            &expanded_path,
            &self.file_pattern,
            &self.search_query,
            self.case_sensitive,
            self.use_regex,
            self.recursive,
            self.file_age_hours,
        );
        let duration = start.elapsed();
        info!(
            "Search completed in {:.2}s: found {} matches in {} files",
            duration.as_secs_f64(),
            self.results.iter().map(|r| r.matches.len()).sum::<usize>(),
            self.results.len()
        );
        self.searching = false;
        self.selected_result = None;
        self.last_search_time = Instant::now();

        // Initialize all headers as expanded for new search
        self.collapsing_state.clear();
        for i in 0..self.results.len() {
            self.collapsing_state.insert(i, true);
        }
    }
}

impl eframe::App for VisGrepApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process keyboard input and handle navigation commands
        if let Some(command) = self.input_handler.process_input(ctx) {
            self.handle_navigation_command(command);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with title and status
            self.render_header(ui);
            ui.separator();

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
                ui.add(egui::TextEdit::singleline(&mut self.results_filter).desired_width(300.0));
                if ui.small_button("Clear").clicked() {
                    self.results_filter.clear();
                }

                ui.separator();

                if ui.button("Expand All").clicked() {
                    for i in 0..self.results.len() {
                        self.collapsing_state.insert(i, true);
                    }
                }
                if ui.button("Collapse All").clicked() {
                    for i in 0..self.results.len() {
                        self.collapsing_state.insert(i, false);
                    }
                }
            });
            ui.separator();

            // Main content area - results and preview
            let available_height = ui.available_height();

            // Results panel (40% of available height)
            egui::ScrollArea::vertical()
                .id_salt("results_scroll")
                .max_height(available_height * 0.4)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.searching {
                        ui.label("Searching...");
                    } else if self.results.is_empty() && !self.search_query.is_empty() {
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

            ui.separator();

            // Preview panel (remaining space)
            ui.label("Preview:");

            let remaining_height = ui.available_height();

            let mut scroll_area = egui::ScrollArea::vertical()
                .id_salt("preview_scroll")
                .max_height(remaining_height)
                .auto_shrink([false, false]);

            // Only force scroll position when a new match is selected
            if self.should_scroll_to_match {
                scroll_area = scroll_area.scroll_offset(egui::Vec2::new(0.0, self.preview_scroll_offset));
                self.should_scroll_to_match = false; // Reset flag after applying
            }

            scroll_area.show(ui, |ui| {
                self.render_preview(ui);
            });

            ui.separator();

            // Status bar
            self.render_status_bar(ui);
        });

        // Debounced search handling
        if self.pending_search
            && self.last_search_time.elapsed() > std::time::Duration::from_millis(500)
            && !self.search_query.is_empty() {
                self.perform_search();
                self.pending_search = false;
            }

        ctx.request_repaint();
    }
}

impl VisGrepApp {
    fn select_match(
        &mut self,
        result_id: usize,
        file_path: &std::path::Path,
        line_number: usize,
    ) {
        self.selected_result = Some(result_id);
        self.preview.load_file(file_path, line_number);

        // Calculate scroll offset to center the target line in viewport
        if let Some(target_line_idx) = self.preview.target_line_in_preview {
            let line_height = 14.0; // egui code editor default line height
            let lines_above_target = 10;
            let scroll_to_line = target_line_idx.saturating_sub(lines_above_target);
            self.preview_scroll_offset = scroll_to_line as f32 * line_height;
            self.should_scroll_to_match = true; // Flag that we want to scroll
            info!("Match selected: file line {}, preview line index {}, scroll to line {} (show {} lines above), offset {}px",
                  line_number, target_line_idx, scroll_to_line, lines_above_target, self.preview_scroll_offset);
        }
    }

    fn select_match_with_keyboard(
        &mut self,
        result_id: usize,
        file_path: &std::path::Path,
        line_number: usize,
    ) {
        self.select_match(result_id, file_path, line_number);
        self.scroll_to_selected_result = true; // Flag to scroll results panel
    }

    fn select_next_match(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let current_id = self.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;
        let current_match_idx = current_id % 10000;

        // Try next match in current file
        if current_file_idx < self.results.len()
            && current_match_idx + 1 < self.results[current_file_idx].matches.len() {
                let next_id = current_file_idx * 10000 + current_match_idx + 1;
                let file_path = self.results[current_file_idx].file_path.clone();
                let line_number =
                    self.results[current_file_idx].matches[current_match_idx + 1].line_number;
                self.select_match_with_keyboard(next_id, &file_path, line_number);
                return;
            }

        // Move to first match in next file
        for file_idx in (current_file_idx + 1)..self.results.len() {
            if !self.results[file_idx].matches.is_empty() {
                let next_id = file_idx * 10000;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(next_id, &file_path, line_number);
                return;
            }
        }

        // Wrap to first match
        if !self.results.is_empty() && !self.results[0].matches.is_empty() {
            let file_path = self.results[0].file_path.clone();
            let line_number = self.results[0].matches[0].line_number;
            self.select_match_with_keyboard(0, &file_path, line_number);
        }
    }

    fn handle_navigation_command(&mut self, command: NavigationCommand) {
        match command {
            NavigationCommand::NextMatch => self.select_next_match(),
            NavigationCommand::PreviousMatch => self.select_previous_match(),
            NavigationCommand::FirstMatch => self.select_first_match(),
            NavigationCommand::LastMatch => self.select_last_match(),
            NavigationCommand::NextMatchWithCount(count) => {
                for _ in 0..count {
                    self.select_next_match();
                }
            }
            NavigationCommand::PreviousMatchWithCount(count) => {
                for _ in 0..count {
                    self.select_previous_match();
                }
            }
            NavigationCommand::FirstMatchInCurrentFile => self.select_first_match_in_current_file(),
            NavigationCommand::LastMatchInCurrentFile => self.select_last_match_in_current_file(),
            NavigationCommand::NextFile => self.select_next_file(),
            NavigationCommand::PreviousFile => self.select_previous_file(),
            NavigationCommand::NextFileWithCount(count) => {
                for _ in 0..count {
                    self.select_next_file();
                }
            }
            NavigationCommand::PreviousFileWithCount(count) => {
                for _ in 0..count {
                    self.select_previous_file();
                }
            }
            NavigationCommand::YankMatchedLine => self.yank_matched_line(),
            NavigationCommand::OpenInExplorer => self.open_in_explorer(),
            NavigationCommand::SetMark(ch) => self.set_mark(ch),
            NavigationCommand::GotoMark(ch) => self.goto_mark(ch),
        }
    }

    fn set_mark(&mut self, ch: char) {
        if let Some(result_id) = self.selected_result {
            self.marks.insert(ch, result_id);
            info!("Set mark '{}' at result {}", ch, result_id);
        } else {
            info!("No result selected to mark");
        }
    }

    fn goto_mark(&mut self, ch: char) {
        if let Some(&result_id) = self.marks.get(&ch) {
            let file_idx = result_id / 10000;
            let match_idx = result_id % 10000;

            if file_idx < self.results.len() && match_idx < self.results[file_idx].matches.len() {
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[match_idx].line_number;
                self.select_match_with_keyboard(result_id, &file_path, line_number);
                info!("Jumped to mark '{}'", ch);
            } else {
                info!("Mark '{}' points to invalid result", ch);
            }
        } else {
            info!("Mark '{}' not set", ch);
        }
    }

    fn open_in_explorer(&self) {
        if self.results.is_empty() {
            info!("No results to open");
            return;
        }

        let current_id = self.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        if current_file_idx >= self.results.len() {
            info!("Invalid file index");
            return;
        }

        let file_path = &self.results[current_file_idx].file_path;

        #[cfg(target_os = "windows")]
        {
            // On Windows, use 'explorer /select,' to open Explorer and select the file
            if let Err(e) = std::process::Command::new("explorer")
                .args(&["/select,", &file_path.to_string_lossy()])
                .spawn()
            {
                info!("Failed to open explorer: {}", e);
            } else {
                info!("Opened file in Explorer: {:?}", file_path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, use 'open -R' to reveal in Finder
            if let Err(e) = std::process::Command::new("open")
                .args(&["-R", &file_path.to_string_lossy()])
                .spawn()
            {
                info!("Failed to open Finder: {}", e);
            } else {
                info!("Opened file in Finder: {:?}", file_path);
            }
        }

        #[cfg(target_os = "linux")]
        {
            // On Linux, try various file managers
            let file_managers = [
                ("nautilus", vec!["--select"]),
                ("dolphin", vec!["--select"]),
                ("nemo", vec![]),
                ("thunar", vec![]),
                ("xdg-open", vec![]),
            ];

            let parent_dir = file_path.parent().unwrap_or(file_path.as_ref());
            let mut opened = false;

            for (manager, args) in &file_managers {
                let mut cmd = std::process::Command::new(manager);
                for arg in args {
                    cmd.arg(arg);
                }
                cmd.arg(file_path.to_string_lossy().to_string());

                if cmd.spawn().is_ok() {
                    info!("Opened file with {}: {:?}", manager, file_path);
                    opened = true;
                    break;
                }
            }

            if !opened {
                // Fallback: just open the parent directory
                if let Err(e) = std::process::Command::new("xdg-open")
                    .arg(parent_dir.to_string_lossy().to_string())
                    .spawn()
                {
                    info!("Failed to open file manager: {}", e);
                } else {
                    info!("Opened parent directory: {:?}", parent_dir);
                }
            }
        }
    }

    fn yank_matched_line(&mut self) {
        if let Some(matched_line) = &self.preview.matched_line_text {
            match Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(matched_line.clone()) {
                    Ok(_) => info!(
                        "Yanked matched line ({} chars) to clipboard",
                        matched_line.len()
                    ),
                    Err(e) => info!("Failed to yank matched line to clipboard: {}", e),
                },
                Err(e) => info!("Failed to access clipboard: {}", e),
            }
        } else {
            info!("No matched line to yank");
        }
    }

    fn select_first_match(&mut self) {
        if self.results.is_empty() {
            return;
        }

        // Find first file with matches
        for file_idx in 0..self.results.len() {
            if !self.results[file_idx].matches.is_empty() {
                let result_id = file_idx * 10000;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(result_id, &file_path, line_number);
                return;
            }
        }
    }

    fn select_last_match(&mut self) {
        if self.results.is_empty() {
            return;
        }

        // Find last file with matches, and last match in that file
        for file_idx in (0..self.results.len()).rev() {
            if !self.results[file_idx].matches.is_empty() {
                let last_match_idx = self.results[file_idx].matches.len() - 1;
                let result_id = file_idx * 10000 + last_match_idx;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[last_match_idx].line_number;
                self.select_match_with_keyboard(result_id, &file_path, line_number);
                return;
            }
        }
    }

    fn select_first_match_in_current_file(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let current_id = self.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        if current_file_idx < self.results.len()
            && !self.results[current_file_idx].matches.is_empty()
        {
            let result_id = current_file_idx * 10000;
            let file_path = self.results[current_file_idx].file_path.clone();
            let line_number = self.results[current_file_idx].matches[0].line_number;
            self.select_match_with_keyboard(result_id, &file_path, line_number);
        }
    }

    fn select_last_match_in_current_file(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let current_id = self.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        if current_file_idx < self.results.len()
            && !self.results[current_file_idx].matches.is_empty()
        {
            let last_match_idx = self.results[current_file_idx].matches.len() - 1;
            let result_id = current_file_idx * 10000 + last_match_idx;
            let file_path = self.results[current_file_idx].file_path.clone();
            let line_number = self.results[current_file_idx].matches[last_match_idx].line_number;
            self.select_match_with_keyboard(result_id, &file_path, line_number);
        }
    }

    fn select_next_file(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let current_id = self.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        // Move to first match in next file
        for file_idx in (current_file_idx + 1)..self.results.len() {
            if !self.results[file_idx].matches.is_empty() {
                let next_id = file_idx * 10000;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(next_id, &file_path, line_number);
                return;
            }
        }

        // Wrap to first file
        for file_idx in 0..self.results.len() {
            if !self.results[file_idx].matches.is_empty() {
                let next_id = file_idx * 10000;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(next_id, &file_path, line_number);
                return;
            }
        }
    }

    fn select_previous_file(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let current_id = self.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;

        // Move to first match in previous file
        for file_idx in (0..current_file_idx).rev() {
            if !self.results[file_idx].matches.is_empty() {
                let prev_id = file_idx * 10000;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(prev_id, &file_path, line_number);
                return;
            }
        }

        // Wrap to last file
        for file_idx in (0..self.results.len()).rev() {
            if !self.results[file_idx].matches.is_empty() {
                let prev_id = file_idx * 10000;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[0].line_number;
                self.select_match_with_keyboard(prev_id, &file_path, line_number);
                return;
            }
        }
    }

    fn select_previous_match(&mut self) {
        if self.results.is_empty() {
            return;
        }

        let current_id = self.selected_result.unwrap_or(0);
        let current_file_idx = current_id / 10000;
        let current_match_idx = current_id % 10000;

        // Try previous match in current file
        if current_match_idx > 0 {
            let prev_id = current_file_idx * 10000 + current_match_idx - 1;
            let file_path = self.results[current_file_idx].file_path.clone();
            let line_number =
                self.results[current_file_idx].matches[current_match_idx - 1].line_number;
            self.select_match_with_keyboard(prev_id, &file_path, line_number);
            return;
        }

        // Move to last match in previous file
        for file_idx in (0..current_file_idx).rev() {
            if !self.results[file_idx].matches.is_empty() {
                let last_match_idx = self.results[file_idx].matches.len() - 1;
                let prev_id = file_idx * 10000 + last_match_idx;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[last_match_idx].line_number;
                self.select_match_with_keyboard(prev_id, &file_path, line_number);
                return;
            }
        }

        // Wrap to last match in last file
        for file_idx in (0..self.results.len()).rev() {
            if !self.results[file_idx].matches.is_empty() {
                let last_match_idx = self.results[file_idx].matches.len() - 1;
                let last_id = file_idx * 10000 + last_match_idx;
                let file_path = self.results[file_idx].file_path.clone();
                let line_number = self.results[file_idx].matches[last_match_idx].line_number;
                self.select_match_with_keyboard(last_id, &file_path, line_number);
                return;
            }
        }
    }

    fn render_results(&mut self, ui: &mut egui::Ui) {
        let filter = self.results_filter.to_lowercase();
        let mut clicked_match: Option<(usize, std::path::PathBuf, usize)> = None;
        let should_scroll = self.scroll_to_selected_result;
        self.scroll_to_selected_result = false; // Reset flag

        for (file_idx, result) in self.results.iter().enumerate() {
            let file_name = result
                .file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Apply filename filter
            if !filter.is_empty() && !file_name.to_lowercase().contains(&filter) {
                continue;
            }

            // Get current open state, default to true if not set
            let is_open = *self.collapsing_state.get(&file_idx).unwrap_or(&true);

            let header_id = ui.make_persistent_id(format!("header_{}", file_idx));

            // Load the state from egui's storage (respects user clicks)
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                header_id,
                is_open,
            );

            // Only force the state if our tracked state differs from egui's state
            // This allows user clicks to work, but also allows Expand/Collapse All buttons to work
            if state.is_open() != is_open {
                state.set_open(is_open);
                state.store(ui.ctx());
            }

            state
                .show_header(ui, |ui| {
                    ui.label(format!("{} ({} matches)", file_name, result.matches.len()));
                })
                .body(|ui| {
                    for (match_idx, m) in result.matches.iter().enumerate() {
                        let result_id = file_idx * 10000 + match_idx;
                        let is_selected = self.selected_result == Some(result_id);

                        let label = format!("  Line {}: {}", m.line_number, m.line_text.trim());

                        let response = ui.selectable_label(is_selected, label);

                        if response.clicked() {
                            clicked_match =
                                Some((result_id, result.file_path.clone(), m.line_number));
                        }

                        // Scroll to this item if it's selected and we should scroll
                        if is_selected && should_scroll {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                });

            // Re-load state to get updated open/close status after user interaction
            let updated_state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                header_id,
                is_open,
            );
            self.collapsing_state
                .insert(file_idx, updated_state.is_open());
        }

        // Handle match selection after iteration is complete
        if let Some((result_id, file_path, line_number)) = clicked_match {
            self.select_match(result_id, &file_path, line_number);
        }
    }

    fn render_preview(&mut self, ui: &mut egui::Ui) {
        if let Some(preview_text) = &self.preview.content {
            // Check if we should try syntax highlighting based on selected result
            let should_highlight = if let Some(selected_id) = self.selected_result {
                let file_idx = selected_id / 10000;
                self.results
                    .get(file_idx)
                    .map(|r| self.should_highlight_file(&r.file_path))
                    .unwrap_or(false)
            } else {
                false
            };

            if should_highlight {
                // Use egui_extras syntax highlighting
                let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                    let mut layout_job = egui_extras::syntax_highlighting::highlight(
                        ui.ctx(),
                        ui.style().as_ref(),
                        &egui_extras::syntax_highlighting::CodeTheme::from_memory(
                            ui.ctx(),
                            ui.style().as_ref(),
                        ),
                        string,
                        "rs", // Default to rust, we can make this smarter later
                    );
                    layout_job.wrap.max_width = wrap_width;
                    ui.fonts(|f| f.layout_job(layout_job))
                };

                ui.add(
                    egui::TextEdit::multiline(&mut preview_text.as_str())
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(100)
                        .layouter(&mut layouter),
                );
            } else {
                // Plain text for non-code files
                // Always use custom rendering to highlight matched line
                self.render_preview_with_highlights(ui, preview_text);
            }
        } else {
            ui.label("Select a result to preview");
        }
    }

    fn render_matched_line_focus(&self, ui: &mut egui::Ui) {
        use egui::{Color32, RichText};

        if let Some(matched_line) = &self.preview.matched_line_text {
            ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

            let highlight_color = Color32::from_rgb(255, 200, 100); // Orange/yellow
            let highlight_bg = Color32::from_rgb(80, 60, 40); // Brown background

            // Use highlight pattern if specified, otherwise use search query
            let pattern_to_use = if !self.fix_highlight_pattern.is_empty() {
                &self.fix_highlight_pattern
            } else {
                &self.search_query
            };

            let has_pattern = !pattern_to_use.is_empty();

            if has_pattern && matched_line.contains(pattern_to_use) {
                // Render with highlighted pattern
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    let parts: Vec<&str> = matched_line.split(pattern_to_use).collect();

                    for (i, part) in parts.iter().enumerate() {
                        if !part.is_empty() {
                            ui.label(*part);
                        }

                        // Add highlighted pattern between parts (except after last part)
                        if i < parts.len() - 1 {
                            ui.label(
                                RichText::new(pattern_to_use)
                                    .color(highlight_color)
                                    .background_color(highlight_bg)
                                    .strong(),
                            );
                        }
                    }
                });
            } else {
                // Just show the line normally
                ui.label(matched_line);
            }
        } else {
            ui.label(
                RichText::new("Select a match to see the line here")
                    .italics()
                    .color(Color32::GRAY),
            );
        }
    }

    fn render_preview_with_highlights(&self, ui: &mut egui::Ui, text: &str) {
        use egui::Color32;

        egui::ScrollArea::neither()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);

                let match_line_bg = Color32::from_rgb(60, 60, 80); // Subtle blue-gray for matched line

                for line in text.lines() {
                    let is_match_line = line.starts_with(">>>");

                    // Apply background color for matched line
                    if is_match_line {
                        let frame = egui::Frame::none()
                            .fill(match_line_bg)
                            .inner_margin(egui::Margin::symmetric(4.0, 2.0));

                        frame.show(ui, |ui| {
                            ui.label(line);
                        });
                    } else {
                        // Regular line
                        ui.label(line);
                    }
                }
            });
    }

    fn should_highlight_file(&self, path: &std::path::Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(
                ext,
                "rs" | "toml"
                    | "js"
                    | "ts"
                    | "tsx"
                    | "jsx"
                    | "py"
                    | "java"
                    | "c"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "go"
                    | "rb"
                    | "php"
                    | "cs"
                    | "swift"
                    | "kt"
                    | "scala"
                    | "sh"
                    | "bash"
                    | "json"
                    | "xml"
                    | "html"
                    | "css"
                    | "md"
                    | "yaml"
                    | "yml"
                    | "sql"
            )
        } else {
            false
        }
    }

    // ============================================================================
    // UI Rendering Functions - Extracted from update()
    // ============================================================================

    /// Render the header with title and status indicators
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading("VisGrep - Fast Search Tool");

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Show pending input state (e.g., "3" or "g")
                let status = self.input_handler.get_status();
                if !status.is_empty() {
                    ui.label(format!("Command: {}", status));
                }

                // Show active marks
                if !self.marks.is_empty() {
                    let marks_str: String = self.marks.keys().collect();
                    ui.label(format!("Marks: {}", marks_str));
                }
            });
        });
    }

    /// Render the highlight pattern field
    fn render_highlight_pattern_field(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Highlight pattern in Matched Line (e.g., 150= or fn):");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.fix_highlight_pattern)
                    .desired_width(150.0)
                    .hint_text("uses search query if empty"),
            );

            // Show active indicator
            let active_pattern = if !self.fix_highlight_pattern.is_empty() {
                &self.fix_highlight_pattern
            } else {
                &self.search_query
            };

            if !active_pattern.is_empty() {
                ui.label(
                    egui::RichText::new(format!("✓ Active: '{}'", active_pattern))
                        .color(egui::Color32::from_rgb(100, 255, 100)),
                );
            }

            if ui.small_button("Clear").clicked() {
                self.fix_highlight_pattern.clear();
            }

            // Log when pattern changes
            if response.changed() {
                info!("Highlight pattern changed to: '{}'", self.fix_highlight_pattern);
            }
        });
    }

    /// Render the search path field with folder presets
    fn render_search_path_field(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search Path:");
            ui.add(egui::TextEdit::singleline(&mut self.search_path).desired_width(350.0));

            // Preset folders dropdown
            egui::ComboBox::from_id_salt("folder_presets")
                .selected_text("📁")
                .width(40.0)
                .show_ui(ui, |ui| {
                    for preset in &self.config.folder_presets {
                        if ui.selectable_label(false, &preset.name).clicked() {
                            self.search_path = Self::expand_tilde(&preset.path);
                            info!("Selected preset: {} -> {}", preset.name, self.search_path);
                        }
                    }
                });

            if ui.button("Current Dir").clicked() {
                if let Ok(cwd) = std::env::current_dir() {
                    self.search_path = cwd.display().to_string();
                }
            }

            if ui.button("Browse...").clicked() {
                match rfd::FileDialog::new().pick_folder() {
                    Some(path) => {
                        self.search_path = path.display().to_string();
                        info!("Selected folder: {}", self.search_path);
                    }
                    None => {
                        info!("Browse dialog cancelled or unavailable");
                    }
                }
            }

            ui.label("File Pattern:");
            ui.add(egui::TextEdit::singleline(&mut self.file_pattern).desired_width(150.0));
            if ui.small_button("Clear").clicked() {
                self.file_pattern.clear();
            }
        });
    }

    /// Render the search query field with patterns dropdown
    fn render_search_query_field(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Search Query:");
            let response =
                ui.add(egui::TextEdit::singleline(&mut self.search_query).desired_width(300.0));

            // Saved patterns dropdown
            if !self.config.saved_patterns.is_empty() {
                self.render_patterns_dropdown(ui);
            }

            // Debounced auto-search: trigger search 500ms after typing stops
            if response.changed() {
                self.pending_search = true;
                self.last_search_time = Instant::now();
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && !self.search_query.is_empty() {
                    self.perform_search();
                }

            ui.checkbox(&mut self.case_sensitive, "Case Sensitive");
            ui.checkbox(&mut self.use_regex, "Regex");
            ui.checkbox(&mut self.recursive, "Recursive");

            if ui.button("Search").clicked() && !self.search_query.is_empty() {
                self.perform_search();
            }
        });
    }

    /// Render the saved patterns dropdown
    fn render_patterns_dropdown(&mut self, ui: &mut egui::Ui) {
        egui::ComboBox::from_id_salt("saved_patterns")
            .selected_text("📝")
            .width(40.0)
            .show_ui(ui, |ui| {
                // Group by category if available
                let mut by_category: std::collections::HashMap<String, Vec<&config::SavedPattern>> =
                    std::collections::HashMap::new();

                for pattern in &self.config.saved_patterns {
                    let cat = if pattern.category.is_empty() {
                        "Other".to_string()
                    } else {
                        pattern.category.clone()
                    };
                    by_category.entry(cat).or_default().push(pattern);
                }

                let mut categories: Vec<_> = by_category.keys().collect();
                categories.sort();

                for category in categories {
                    if let Some(patterns) = by_category.get(category) {
                        if by_category.len() > 1 {
                            ui.label(egui::RichText::new(category).strong());
                            ui.separator();
                        }

                        for pattern in patterns {
                            let label = if pattern.description.is_empty() {
                                pattern.name.clone()
                            } else {
                                pattern.name.to_string()
                            };

                            let mut button = ui.selectable_label(false, label);

                            if !pattern.description.is_empty() {
                                button = button.on_hover_text(&pattern.description);
                            }

                            if button.clicked() {
                                self.search_query = pattern.pattern.clone();
                                info!("Loaded pattern: {} -> {}", pattern.name, pattern.pattern);
                            }
                        }

                        if by_category.len() > 1 {
                            ui.separator();
                        }
                    }
                }
            });
    }

    /// Render file age filter controls
    fn render_file_age_filter(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("File Age:");
            let mut enabled = self.file_age_hours.is_some();
            ui.checkbox(&mut enabled, "Filter by age");

            if enabled {
                let mut hours = self.file_age_hours.unwrap_or(24);
                ui.add(
                    egui::DragValue::new(&mut hours)
                        .speed(1.0)
                        .range(1..=8760),
                );
                ui.label("hours");
                self.file_age_hours = Some(hours);
            } else {
                self.file_age_hours = None;
            }

            if ui.small_button("?").clicked() {
                info!("File Age Filter: Only search files modified within the specified hours");
            }
        });
    }

    /// Render status bar showing search stats
    fn render_status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let total_matches: usize = self.results.iter().map(|r| r.matches.len()).sum();
            let file_count = self.results.len();

            ui.label(format!("Found {} matches in {} files", total_matches, file_count));

            if self.searching {
                ui.spinner();
                ui.label("Searching...");
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("VisGrep starting...");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_title("VisGrep - Fast Search Tool"),
        ..Default::default()
    };

    eframe::run_native(
        "VisGrep",
        native_options,
        Box::new(|_cc| Ok(Box::new(VisGrepApp::default()))),
    )
}
