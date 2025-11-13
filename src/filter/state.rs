use regex::Regex;

#[derive(Debug, Clone)]
pub struct PreviewFilter {
    pub active: bool,
    pub query: String,
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub match_lines: Vec<usize>,
    pub current_match: Option<usize>,
    compiled_regex: Option<Regex>,
}

impl PreviewFilter {
    pub fn new() -> Self {
        Self {
            active: false,
            query: String::new(),
            case_sensitive: false,
            use_regex: false,
            match_lines: Vec::new(),
            current_match: None,
            compiled_regex: None,
        }
    }

    pub fn activate(&mut self) {
        self.active = true;
    }

    pub fn deactivate(&mut self) {
        self.active = false;
        self.query.clear();
        self.match_lines.clear();
        self.current_match = None;
        self.compiled_regex = None;
    }

    pub fn update_query(&mut self, query: String) {
        self.query = query;
        self.parse_query();
        self.match_lines.clear();
        self.current_match = None;
    }

    fn parse_query(&mut self) {
        if self.query.starts_with("C:") {
            self.case_sensitive = true;
            self.use_regex = false;
            self.query = self.query[2..].to_string();
        } else if self.query.starts_with("R:") {
            self.use_regex = true;
            self.case_sensitive = false;
            self.query = self.query[2..].to_string();
            self.compile_regex();
        } else {
            self.case_sensitive = false;
            self.use_regex = false;
        }
    }

    fn compile_regex(&mut self) {
        if self.use_regex {
            match Regex::new(&self.query) {
                Ok(regex) => self.compiled_regex = Some(regex),
                Err(_) => self.compiled_regex = None,
            }
        }
    }

    pub fn matches_line(&self, line: &str) -> bool {
        if self.query.is_empty() {
            return false;
        }

        if self.use_regex {
            if let Some(regex) = &self.compiled_regex {
                regex.is_match(line)
            } else {
                false
            }
        } else if self.case_sensitive {
            line.contains(&self.query)
        } else {
            line.to_lowercase().contains(&self.query.to_lowercase())
        }
    }

    pub fn find_matches(&self, line: &str) -> Vec<(usize, usize)> {
        let mut matches = Vec::new();
        
        if self.query.is_empty() {
            return matches;
        }

        if self.use_regex {
            if let Some(regex) = &self.compiled_regex {
                for m in regex.find_iter(line) {
                    matches.push((m.start(), m.end()));
                }
            }
        } else {
            let search_line = if self.case_sensitive {
                line.to_string()
            } else {
                line.to_lowercase()
            };
            let search_query = if self.case_sensitive {
                self.query.clone()
            } else {
                self.query.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = search_line[start..].find(&search_query) {
                let match_start = start + pos;
                let match_end = match_start + search_query.len();
                matches.push((match_start, match_end));
                start = match_end;
            }
        }

        matches
    }

    pub fn next_match(&mut self) {
        if self.match_lines.is_empty() {
            return;
        }

        match self.current_match {
            Some(idx) => {
                self.current_match = Some((idx + 1) % self.match_lines.len());
            }
            None => {
                self.current_match = Some(0);
            }
        }
    }

    pub fn prev_match(&mut self) {
        if self.match_lines.is_empty() {
            return;
        }

        match self.current_match {
            Some(idx) => {
                if idx > 0 {
                    self.current_match = Some(idx - 1);
                } else {
                    self.current_match = Some(self.match_lines.len() - 1);
                }
            }
            None => {
                self.current_match = Some(self.match_lines.len() - 1);
            }
        }
    }

    pub fn current_match_line(&self) -> Option<usize> {
        if let Some(idx) = self.current_match {
            self.match_lines.get(idx).copied()
        } else {
            None
        }
    }

    pub fn match_stats(&self) -> (usize, usize) {
        (
            self.current_match.unwrap_or(0).saturating_add(1).min(self.match_lines.len()),
            self.match_lines.len()
        )
    }
}

#[derive(Debug, Clone)]
pub struct TreeFilter {
    pub active: bool,
    pub pattern: String,
    pub show_matching_only: bool,
    pub exclude_patterns: Vec<String>,
    pub apply_to_output: bool,
}

impl TreeFilter {
    pub fn new() -> Self {
        Self {
            active: false,
            pattern: String::new(),
            show_matching_only: true,
            exclude_patterns: Vec::new(),
            apply_to_output: true,
        }
    }

    pub fn matches(&self, path: &str) -> bool {
        if self.pattern.is_empty() {
            return true;
        }

        let lower_path = path.to_lowercase();
        let lower_pattern = self.pattern.to_lowercase();

        // Fuzzy match: all characters in pattern must appear in order
        let mut pattern_chars = lower_pattern.chars();
        let mut current_char = pattern_chars.next();

        for path_char in lower_path.chars() {
            if let Some(pc) = current_char {
                if path_char == pc {
                    current_char = pattern_chars.next();
                }
            } else {
                break;
            }
        }

        let matches = current_char.is_none();
        log::trace!("Fuzzy match '{}' against '{}': {}", self.pattern, path, matches);
        matches
    }

    pub fn is_excluded(&self, path: &str) -> bool {
        self.exclude_patterns.iter().any(|pattern| {
            path.contains(pattern)
        })
    }
}