use eframe::egui;
use log::info;
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

    search_engine: SearchEngine,
    results: Vec<SearchResult>,
    selected_result: Option<usize>,
    preview: FilePreview,

    searching: bool,
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

            search_engine: SearchEngine::new(),
            results: Vec::new(),
            selected_result: None,
            preview: FilePreview::new(),

            searching: false,
        }
    }
}

impl VisGrepApp {
    fn perform_search(&mut self) {
        info!("Starting search: path='{}', pattern='{}', query='{}'",
              &self.search_path, &self.file_pattern, &self.search_query);
        self.searching = true;
        let start = Instant::now();
        self.results = self.search_engine.search(
            &self.search_path,
            &self.file_pattern,
            &self.search_query,
            self.case_sensitive,
            self.use_regex,
            self.recursive,
        );
        let duration = start.elapsed();
        info!("Search completed in {:.2}s: found {} matches in {} files",
              duration.as_secs_f64(),
              self.results.iter().map(|r| r.matches.len()).sum::<usize>(),
              self.results.len());
        self.searching = false;
        self.selected_result = None;
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
                    egui::TextEdit::singleline(&mut self.search_query).desired_width(400.0)
                );

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

            ui.separator();

            // Results count
            if !self.results.is_empty() {
                ui.label(format!(
                    "Found {} matches in {} files",
                    self.results.iter().map(|r| r.matches.len()).sum::<usize>(),
                    self.results.len()
                ));
                ui.separator();
            }

            // Split view: results on top, preview on bottom
            let available_height = ui.available_height();

            // Results panel
            egui::ScrollArea::vertical()
                .max_height(available_height * 0.5)
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

            // Preview panel
            ui.heading("Preview");
            egui::ScrollArea::vertical()
                .max_height(available_height * 0.4)
                .show(ui, |ui| {
                    self.render_preview(ui);
                });
        });
    }
}

impl VisGrepApp {
    fn render_results(&mut self, ui: &mut egui::Ui) {
        for (file_idx, result) in self.results.iter().enumerate() {
            let file_name = result.file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            egui::CollapsingHeader::new(format!("{} ({} matches)", file_name, result.matches.len()))
                .default_open(true)
                .show(ui, |ui| {
                    for (match_idx, m) in result.matches.iter().enumerate() {
                        let result_id = file_idx * 10000 + match_idx;
                        let is_selected = self.selected_result == Some(result_id);

                        let label = format!("  Line {}: {}", m.line_number, m.line_text.trim());

                        if ui.selectable_label(is_selected, label).clicked() {
                            self.selected_result = Some(result_id);
                            self.preview.load_file(&result.file_path, m.line_number);
                        }
                    }
                });
        }
    }

    fn render_preview(&mut self, ui: &mut egui::Ui) {
        if let Some(preview_text) = &self.preview.content {
            ui.add(
                egui::TextEdit::multiline(&mut preview_text.as_str())
                    .code_editor()
                    .desired_width(f32::INFINITY)
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
