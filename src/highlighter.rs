use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use std::path::Path;

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn should_highlight(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            matches!(
                ext,
                "rs" | "toml" | "js" | "ts" | "tsx" | "jsx" | "py" | "java" | "c" | "cpp" | "h" | "hpp"
                    | "go" | "rb" | "php" | "cs" | "swift" | "kt" | "scala" | "sh" | "bash" | "json"
                    | "xml" | "html" | "css" | "md" | "yaml" | "yml" | "sql"
            )
        } else {
            false
        }
    }

    pub fn highlight_to_string(&self, text: &str, file_path: &Path) -> String {
        // Try to find syntax based on file extension
        let syntax = self
            .syntax_set
            .find_syntax_for_file(file_path)
            .ok()
            .flatten()
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        // Use a dark theme (Monokai-like)
        let theme = &self.theme_set.themes["base16-ocean.dark"];

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut result = String::new();

        for line in LinesWithEndings::from(text) {
            let ranges = highlighter.highlight_line(line, &self.syntax_set).unwrap();

            // For now, just return the plain text
            // In the future we could add ANSI color codes or convert to rich text
            result.push_str(line);
        }

        result
    }

    pub fn get_theme_name(&self) -> &str {
        "base16-ocean.dark"
    }
}
