use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, Duration};
use walkdir::WalkDir;
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct MatchInfo {
    pub line_number: usize,
    pub line_text: String,
    pub column_start: usize,
    pub column_end: usize,
}

#[derive(Debug)]
pub struct SearchResult {
    pub file_path: PathBuf,
    pub matches: Vec<MatchInfo>,
}

pub struct SearchEngine;

impl SearchEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn search(
        &self,
        search_path: &str,
        file_pattern: &str,
        query: &str,
        case_sensitive: bool,
        use_regex: bool,
        recursive: bool,
        file_age_hours: Option<u64>,
    ) -> Vec<SearchResult> {
        let path = Path::new(search_path);
        if !path.exists() {
            return Vec::new();
        }

        let age_cutoff = file_age_hours.map(|hours| {
            SystemTime::now() - Duration::from_secs(hours * 3600)
        });

        // Collect files matching the pattern
        let files: Vec<PathBuf> = if path.is_file() {
            vec![path.to_path_buf()]
        } else if recursive {
            WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .filter(|e| self.matches_pattern(e.path(), file_pattern))
                .filter(|e| self.matches_age(e.path(), age_cutoff))
                .map(|e| e.path().to_path_buf())
                .collect()
        } else {
            std::fs::read_dir(path)
                .ok()
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                        .filter(|e| self.matches_pattern(&e.path(), file_pattern))
                        .filter(|e| self.matches_age(&e.path(), age_cutoff))
                        .map(|e| e.path())
                        .collect()
                })
                .unwrap_or_default()
        };

        // Search in parallel
        files
            .par_iter()
            .filter_map(|file| self.search_file(file, query, case_sensitive, use_regex))
            .collect()
    }

    fn matches_pattern(&self, path: &Path, pattern: &str) -> bool {
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => return false,
        };

        // Simple glob pattern matching
        if pattern == "*" || pattern.is_empty() {
            return true;
        }

        // Convert simple glob to regex
        let pattern_regex = pattern
            .replace(".", "\\.")
            .replace("*", ".*")
            .replace("?", ".");

        Regex::new(&format!("^{}$", pattern_regex))
            .ok()
            .and_then(|re| Some(re.is_match(file_name)))
            .unwrap_or(false)
    }

    fn matches_age(&self, path: &Path, cutoff: Option<SystemTime>) -> bool {
        let Some(cutoff_time) = cutoff else {
            return true; // No age filter
        };

        // Check file modification time
        if let Ok(metadata) = std::fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                return modified >= cutoff_time;
            }
        }

        false // If we can't get metadata, exclude the file
    }

    fn search_file(
        &self,
        file_path: &Path,
        query: &str,
        case_sensitive: bool,
        use_regex: bool,
    ) -> Option<SearchResult> {
        let regex = if use_regex {
            let pattern = if case_sensitive {
                query.to_string()
            } else {
                format!("(?i){}", query)
            };
            Regex::new(&pattern).ok()?
        } else {
            let escaped = regex::escape(query);
            let pattern = if case_sensitive {
                escaped
            } else {
                format!("(?i){}", escaped)
            };
            Regex::new(&pattern).ok()?
        };

        let file = File::open(file_path).ok()?;
        let reader = BufReader::new(file);

        let mut matches = Vec::new();

        for (line_idx, line) in reader.lines().enumerate() {
            if let Ok(line_text) = line {
                if let Some(mat) = regex.find(&line_text) {
                    matches.push(MatchInfo {
                        line_number: line_idx + 1,
                        line_text: line_text.clone(),
                        column_start: mat.start(),
                        column_end: mat.end(),
                    });
                }
            }
        }

        if !matches.is_empty() {
            Some(SearchResult {
                file_path: file_path.to_path_buf(),
                matches,
            })
        } else {
            None
        }
    }
}
