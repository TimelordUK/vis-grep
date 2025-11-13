use eframe::egui::{self, TextEdit, RichText, TextStyle};
use super::state::TreeFilter;

pub fn render_tree_filter(ui: &mut egui::Ui, filter: &mut TreeFilter) -> bool {
    let mut changed = false;
    
    ui.horizontal(|ui| {
        ui.label("Filter files:");
        
        let response = ui.add(
            TextEdit::singleline(&mut filter.pattern)
                .desired_width(150.0)
                .font(TextStyle::Monospace)
                .hint_text("Type to filter...")
        );
        
        if response.changed() {
            filter.active = !filter.pattern.is_empty();
            changed = true;
        }
        
        if ui.small_button("×").on_hover_text("Clear filter").clicked() {
            filter.pattern.clear();
            filter.active = false;
            changed = true;
        }
        
        if filter.active {
            // Checkbox to apply filter to output
            let checkbox_response = ui.checkbox(&mut filter.apply_to_output, "")
                .on_hover_text("Also filter combined output");
            if checkbox_response.changed() {
                changed = true;
            }
            
            ui.label(
                RichText::new(format!("Output {}", if filter.apply_to_output { "filtered" } else { "all" }))
                    .small()
                    .color(if filter.apply_to_output { 
                        egui::Color32::from_rgb(255, 200, 100) 
                    } else { 
                        egui::Color32::from_gray(128) 
                    })
            );
        }
    });
    
    changed
}

fn count_visible_files(filter: &TreeFilter) -> usize {
    // This is a placeholder - the actual count should come from the filtered file list
    // Will be updated when we have access to the file list
    0
}

pub fn is_file_visible(filter: &TreeFilter, path: &str, display_name: &str) -> bool {
    if !filter.active || filter.pattern.is_empty() {
        return true;
    }
    
    // Check if excluded
    if filter.is_excluded(path) {
        log::debug!("File excluded by pattern: {}", path);
        return false;
    }
    
    // Check if matches pattern (try both path and display name)
    let path_match = filter.matches(path);
    let name_match = filter.matches(display_name);
    let visible = path_match || name_match;
    
    log::debug!(
        "Filter '{}' on '{}' (name: '{}'): path_match={}, name_match={}, visible={}",
        filter.pattern, path, display_name, path_match, name_match, visible
    );
    
    visible
}