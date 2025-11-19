use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::theme::Theme;
use crate::log_parser::{LogColorScheme, LogColorPreset};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderPreset {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPattern {
    pub name: String,
    pub pattern: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// The pager command to run inside the terminal (e.g., "less", "more", "bat")
    #[serde(default = "default_pager")]
    pub pager: String,
    #[serde(default)]
    pub pager_args: Vec<String>,
}

fn default_pager() -> String {
    if cfg!(windows) {
        "more".to_string()
    } else {
        "less".to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFormatConfig {
    /// Custom regex patterns for log level detection
    /// Format: (pattern, level_name) where level_name is: TRACE, DEBUG, INFO, WARN, ERROR, FATAL
    #[serde(default)]
    pub custom_patterns: Vec<(String, String)>,

    /// Color preset: Vibrant (default, colorful), Subtle (muted), or Monochrome (gray with red errors)
    #[serde(default)]
    pub color_preset: LogColorPreset,

    /// Color scheme for log levels (auto-set from preset, but can be customized)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_colors: Option<LogColorScheme>,
}

impl LogFormatConfig {
    /// Get the effective color scheme (custom colors or preset)
    pub fn get_color_scheme(&self) -> LogColorScheme {
        self.custom_colors
            .clone()
            .unwrap_or_else(|| LogColorScheme::from_preset(self.color_preset))
    }
}

impl Default for LogFormatConfig {
    fn default() -> Self {
        Self {
            custom_patterns: vec![],
            color_preset: LogColorPreset::Vibrant,
            custom_colors: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPreferences {
    /// Default font size for UI elements
    #[serde(default = "default_font_size")]
    pub font_size: f32,

    /// Tail mode file polling interval in milliseconds
    #[serde(default = "default_poll_interval")]
    pub poll_interval_ms: u64,
    
    /// Maximum number of lines to keep in tail mode buffer
    #[serde(default = "default_max_buffer_lines")]
    pub max_buffer_lines: usize,
}

fn default_font_size() -> f32 {
    14.0
}

fn default_poll_interval() -> u64 {
    250
}

fn default_max_buffer_lines() -> usize {
    10000
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            font_size: default_font_size(),
            poll_interval_ms: default_poll_interval(),
            max_buffer_lines: default_max_buffer_lines(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub folder_presets: Vec<FolderPreset>,
    #[serde(default)]
    pub saved_patterns: Vec<SavedPattern>,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub editor: Option<EditorConfig>,
    #[serde(default)]
    pub terminal: Option<TerminalConfig>,
    #[serde(default)]
    pub log_format: LogFormatConfig,
    #[serde(default)]
    pub ui: UiPreferences,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            folder_presets: vec![
                FolderPreset {
                    name: "Home".to_string(),
                    path: "~/".to_string(),
                },
                FolderPreset {
                    name: "Current Directory".to_string(),
                    path: ".".to_string(),
                },
            ],
            saved_patterns: vec![],
            theme: Theme::default(),
            editor: None,
            terminal: None,
            log_format: LogFormatConfig::default(),
            ui: UiPreferences::default(),
        }
    }
}

impl Config {
    /// Get the config file path
    /// - Windows: %APPDATA%\vis-grep\config.yaml
    /// - Linux/Mac: ~/.config/vis-grep/config.yaml
    pub fn config_path() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            if let Some(app_data) = std::env::var_os("APPDATA") {
                let mut path = PathBuf::from(app_data);
                path.push("vis-grep");
                path.push("config.yaml");
                return Some(path);
            } else if let Some(user_profile) = std::env::var_os("USERPROFILE") {
                let mut path = PathBuf::from(user_profile);
                path.push(".config");
                path.push("vis-grep");
                path.push("config.yaml");
                return Some(path);
            }
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            if let Some(home) = std::env::var_os("HOME") {
                let mut path = PathBuf::from(home);
                path.push(".config");
                path.push("vis-grep");
                path.push("config.yaml");
                return Some(path);
            }
        }
        
        None
    }

    /// Load config from file, or create default if not exists
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => match serde_yaml::from_str(&content) {
                        Ok(config) => {
                            info!("Loaded config from {:?}", path);
                            let cfg: Config = config;
                            info!("Terminal config loaded: {:?}", cfg.terminal.is_some());
                            if let Some(ref term) = cfg.terminal {
                                info!("Terminal: command={}, pager={}", term.command, term.pager);
                            }
                            return cfg;
                        }
                        Err(e) => {
                            warn!("Failed to parse config file: {}", e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read config file: {}", e);
                    }
                }
            } else {
                info!("Config file not found at {:?}, using defaults", path);
            }
        }

        Self::default()
    }

    /// Save config to file
    pub fn save(&self) -> Result<(), String> {
        if let Some(path) = Self::config_path() {
            // Create directory if it doesn't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create config directory: {}", e))?;
            }

            let yaml = serde_yaml::to_string(self)
                .map_err(|e| format!("Failed to serialize config: {}", e))?;

            fs::write(&path, yaml).map_err(|e| format!("Failed to write config file: {}", e))?;

            info!("Saved config to {:?}", path);
            Ok(())
        } else {
            Err("Could not determine config path".to_string())
        }
    }

    /// Create an example config file
    pub fn create_example() -> Result<(), String> {
        let example = Config {
            folder_presets: vec![
                FolderPreset {
                    name: "Logs".to_string(),
                    path: "~/logs".to_string(),
                },
                FolderPreset {
                    name: "FIX Messages".to_string(),
                    path: "~/work/fix-logs".to_string(),
                },
                FolderPreset {
                    name: "Nvim Config".to_string(),
                    path: "~/.config/nvim/lua/plugins".to_string(),
                },
                FolderPreset {
                    name: "Project".to_string(),
                    path: "~/dev/myproject".to_string(),
                },
            ],
            saved_patterns: vec![
                SavedPattern {
                    name: "Execution Report".to_string(),
                    pattern: "35=8".to_string(),
                    description: "MsgType = Execution Report".to_string(),
                    category: "FIX".to_string(),
                },
                SavedPattern {
                    name: "Error".to_string(),
                    pattern: "(?i)error".to_string(),
                    description: "Case-insensitive error messages".to_string(),
                    category: "Errors".to_string(),
                },
            ],
            theme: Theme::default(),
            editor: Some(EditorConfig {
                command: if cfg!(windows) {
                    "notepad".to_string()
                } else {
                    "code".to_string()
                },
                args: vec![],
            }),
            terminal: Some(TerminalConfig {
                command: if cfg!(windows) {
                    "pwsh".to_string()  // PowerShell Core
                } else if cfg!(target_os = "macos") {
                    "Terminal".to_string()  // macOS Terminal.app
                } else {
                    "gnome-terminal".to_string()  // Common Linux terminal
                },
                args: if cfg!(windows) {
                    vec!["-NoExit".to_string(), "-Command".to_string()]
                } else if cfg!(target_os = "macos") {
                    vec![]  // Terminal.app needs special handling
                } else {
                    vec!["--".to_string()]  // Most Linux terminals use -- before command
                },
                pager: if cfg!(windows) {
                    "more".to_string()
                } else {
                    "less".to_string()
                },
                pager_args: if cfg!(windows) {
                    vec![]
                } else {
                    vec!["-R".to_string()]  // -R for color support in less
                },
            }),
            log_format: LogFormatConfig::default(),
            ui: UiPreferences::default(),
        };

        example.save()
    }
}
