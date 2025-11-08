use log::info;
use memmap2::Mmap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct FilePreview {
    pub content: Option<String>,
    pub target_line_in_preview: Option<usize>, // Which line in the preview content has the >>>
}

impl FilePreview {
    pub fn new() -> Self {
        Self {
            content: None,
            target_line_in_preview: None,
        }
    }

    /// Load a preview window around the specified line number
    /// For performance, we only load a window of lines around the target
    pub fn load_file(&mut self, path: &Path, target_line: usize) {
        self.content = None;
        self.target_line_in_preview = None;

        match self.load_preview_fast(path, target_line) {
            Ok((text, preview_line)) => {
                let total_lines = text.lines().count();
                info!("Preview loaded: target_line={}, preview_line_index={}, total_preview_lines={}",
                      target_line, preview_line, total_lines);
                self.content = Some(text);
                self.target_line_in_preview = Some(preview_line);
            }
            Err(e) => {
                info!("Error loading preview for {:?}: {}", path, e);
                self.content = Some(format!("Error loading preview for {:?}", path));
                self.target_line_in_preview = None;
            }
        }
    }

    /// Fast preview loading using buffered reading
    /// Shows context_lines before and after the target line
    /// Returns (preview_text, line_number_in_preview_where_target_is)
    fn load_preview_fast(&self, path: &Path, target_line: usize) -> std::io::Result<(String, usize)> {
        let context_lines = 50; // Show 50 lines before and after for better context
        let start_line = target_line.saturating_sub(context_lines);
        let end_line = target_line + context_lines;

        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        // For small files (< 10MB), just read the whole thing
        if file_size < 10 * 1024 * 1024 {
            let reader = BufReader::new(file);
            let mut preview_line_idx = 0;
            let mut target_preview_line = 0;

            let lines: Vec<String> = reader
                .lines()
                .enumerate()
                .filter(|(idx, _)| *idx >= start_line && *idx <= end_line)
                .filter_map(|(idx, line)| {
                    line.ok().map(|l| {
                        let formatted = if idx + 1 == target_line {
                            target_preview_line = preview_line_idx;
                            format!(">>> {:4} | {}", idx + 1, l)
                        } else {
                            format!("    {:4} | {}", idx + 1, l)
                        };
                        preview_line_idx += 1;
                        formatted
                    })
                })
                .collect();

            return Ok((lines.join("\n"), target_preview_line));
        }

        // For large files, use memory mapping
        self.load_preview_mmap(path, target_line, context_lines)
    }

    fn load_preview_mmap(&self, path: &Path, target_line: usize, context_lines: usize) -> std::io::Result<(String, usize)> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        let start_line = target_line.saturating_sub(context_lines);
        let end_line = target_line + context_lines;

        let mut current_line = 1;
        let mut result = Vec::new();
        let mut line_start = 0;
        let mut target_preview_line = 0;

        for (pos, &byte) in mmap.iter().enumerate() {
            if byte == b'\n' {
                if current_line >= start_line && current_line <= end_line {
                    let line_bytes = &mmap[line_start..pos];
                    if let Ok(line_str) = std::str::from_utf8(line_bytes) {
                        if current_line == target_line {
                            target_preview_line = result.len();
                            result.push(format!(">>> {:4} | {}", current_line, line_str));
                        } else {
                            result.push(format!("    {:4} | {}", current_line, line_str));
                        }
                    }
                }

                current_line += 1;
                line_start = pos + 1;

                if current_line > end_line {
                    break;
                }
            }
        }

        Ok((result.join("\n"), target_preview_line))
    }
}
