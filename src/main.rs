use eframe::egui;
use log::info;
use std::collections::HashMap;
use std::time::Instant;

mod search;
mod preview;

use search::{SearchEngine, SearchResult};
use preview::FilePreview;

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
}

impl Default for VisGrepApp {
    fn default() -> Self {
        Self {
            search_path: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
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
        }
    }
}

impl VisGrepApp {
    fn perform_search(&mut self) {
        info!("Starting search: path='{}', pattern='{}', query='{}', file_age={:?}hrs",
              &self.search_path, &self.file_pattern, &self.search_query, &self.file_age_hours);
        self.searching = true;
        self.pending_search = false;
        let start = Instant::now();
        self.results = self.search_engine.search(
            &self.search_path,
            &self.file_pattern,
            &self.search_query,
            self.case_sensitive,
            self.use_regex,
            self.recursive,
            self.file_age_hours,
        );
        let duration = start.elapsed();
        info!("Search completed in {:.2}s: found {} matches in {} files",
              duration.as_secs_f64(),
              self.results.iter().map(|r| r.matches.len()).sum::<usize>(),
              self.results.len());
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
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("VisGrep - Fast Search Tool");
            ui.separator();

            // Search parameters
            ui.horizontal(|ui| {
                ui.label("Search Path:");
                ui.add(egui::TextEdit::singleline(&mut self.search_path).desired_width(400.0));

                ui.label("File Pattern:");
                ui.add(egui::TextEdit::singleline(&mut self.file_pattern).desired_width(150.0));
                if ui.small_button("Clear").clicked() {
                    self.file_pattern.clear();
                }
            });

            ui.separator();

            // Search query
            ui.horizontal(|ui| {
                ui.label("Search Query:");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query).desired_width(300.0)
                );

                // Debounced auto-search: trigger search 500ms after typing stops
                if response.changed() {
                    self.pending_search = true;
                    self.last_search_time = Instant::now();
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if !self.search_query.is_empty() {
                        self.perform_search();
                    }
                }

                ui.checkbox(&mut self.case_sensitive, "Case Sensitive");
                ui.checkbox(&mut self.use_regex, "Regex");
                ui.checkbox(&mut self.recursive, "Recursive");

                if ui.button("Search").clicked() && !self.search_query.is_empty() {
                    self.perform_search();
                }
            });

            // File age filter
            ui.horizontal(|ui| {
                ui.label("File Age:");
                let mut enabled = self.file_age_hours.is_some();
                ui.checkbox(&mut enabled, "Only files modified in last");

                if enabled && self.file_age_hours.is_none() {
                    self.file_age_hours = Some(24);
                } else if !enabled {
                    self.file_age_hours = None;
                }

                if let Some(ref mut hours) = self.file_age_hours {
                    ui.add(egui::DragValue::new(hours).suffix(" hours").speed(1.0).clamp_range(1..=8760));
                }
            });

            ui.separator();

            // Check for debounced search
            if self.pending_search && !self.search_query.is_empty() {
                let elapsed = self.last_search_time.elapsed();
                if elapsed.as_millis() > 500 {
                    self.perform_search();
                } else {
                    // Request repaint to check again
                    ctx.request_repaint();
                }
            }

            // Results controls and filter
            if !self.results.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Found {} matches in {} files",
                        self.results.iter().map(|r| r.matches.len()).sum::<usize>(),
                        self.results.len()
                    ));

                    ui.separator();

                    ui.label("Filter results:");
                    ui.add(egui::TextEdit::singleline(&mut self.results_filter)
                        .hint_text("filename filter...")
                        .desired_width(150.0));

                    if ui.small_button("Clear").clicked() {
                        self.results_filter.clear();
                    }

                    ui.separator();

                    if ui.button("Expand All").clicked() {
                        info!("Expand All clicked - expanding {} results", self.results.len());
                        for i in 0..self.results.len() {
                            self.collapsing_state.insert(i, true);
                        }
                        info!("Collapsing state: {:?}", self.collapsing_state);
                    }
                    if ui.button("Collapse All").clicked() {
                        info!("Collapse All clicked - collapsing {} results", self.results.len());
                        for i in 0..self.results.len() {
                            self.collapsing_state.insert(i, false);
                        }
                        info!("Collapsing state: {:?}", self.collapsing_state);
                    }
                });
                ui.separator();
            }

            // Split view: results on top, preview on bottom
            let available_height = ui.available_height();

            // Results panel (50% of available height)
            egui::ScrollArea::vertical()
                .id_source("results_scroll") // Give it a unique ID
                .max_height(available_height * 0.5)
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

            // Preview panel (take remaining space)
            ui.heading("Preview");
            let remaining_height = ui.available_height();

            // Use stored scroll offset
            egui::ScrollArea::vertical()
                .id_source("preview_scroll") // Give it a unique ID
                .max_height(remaining_height)
                .auto_shrink([false, false])
                .scroll_offset(egui::Vec2::new(0.0, self.preview_scroll_offset))
                .show(ui, |ui| {
                    self.render_preview(ui);
                });
        });
    }
}

impl VisGrepApp {
    fn render_results(&mut self, ui: &mut egui::Ui) {
        let filter = self.results_filter.to_lowercase();

        for (file_idx, result) in self.results.iter().enumerate() {
            let file_name = result.file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Apply filename filter
            if !filter.is_empty() && !file_name.to_lowercase().contains(&filter) {
                continue;
            }

            // Get current open state, default to true if not set
            let is_open = *self.collapsing_state.get(&file_idx).unwrap_or(&true);

            let header_id = ui.make_persistent_id(format!("header_{}", file_idx));

            // Use CollapsingState to control open/close state
            let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                header_id,
                is_open,
            );
            state.set_open(is_open);
            state.store(ui.ctx());

            state
                .show_header(ui, |ui| {
                    ui.label(format!("{} ({} matches)", file_name, result.matches.len()));
                })
                .body(|ui| {
                    for (match_idx, m) in result.matches.iter().enumerate() {
                        let result_id = file_idx * 10000 + match_idx;
                        let is_selected = self.selected_result == Some(result_id);

                        let label = format!("  Line {}: {}", m.line_number, m.line_text.trim());

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selected_result = Some(result_id);
                            self.preview.load_file(&result.file_path, m.line_number);

                            // Calculate scroll offset to center the target line in viewport
                            // The target line should be at index ~50 in the preview (we load 50 lines before/after)
                            if let Some(target_line_idx) = self.preview.target_line_in_preview {
                                // Measure actual line height from egui's TextEdit
                                let line_height = 14.0; // egui code editor default line height
                                // We want the target line in the middle of viewport
                                // Show about 10 lines above it
                                let lines_above_target = 10;
                                let scroll_to_line = target_line_idx.saturating_sub(lines_above_target);
                                self.preview_scroll_offset = scroll_to_line as f32 * line_height;
                                info!("Result clicked: file line {}, preview line index {}, scroll to line {} (show {} lines above), offset {}px, line_height={}",
                                      m.line_number, target_line_idx, scroll_to_line, lines_above_target, self.preview_scroll_offset, line_height);
                            }
                        }
                    }
                });
        }
    }

    fn render_preview(&mut self, ui: &mut egui::Ui) {
        if let Some(preview_text) = &self.preview.content {
            // Use monospaced font and fill available space
            ui.add(
                egui::TextEdit::multiline(&mut preview_text.as_str())
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(100) // Request many rows to fill space
            );
        } else {
            ui.label("Select a result to preview");
        }
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
