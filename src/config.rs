use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderPreset {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub folder_presets: Vec<FolderPreset>,
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
        }
    }
}

impl Config {
    /// Get the config file path (~/.config/vis-grep/config.yaml)
    pub fn config_path() -> Option<PathBuf> {
        if let Some(home) = std::env::var_os("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".config");
            path.push("vis-grep");
            path.push("config.yaml");
            Some(path)
        } else {
            None
        }
    }

    /// Load config from file, or create default if not exists
    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => match serde_yaml::from_str(&content) {
                        Ok(config) => {
                            info!("Loaded config from {:?}", path);
                            return config;
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

            fs::write(&path, yaml)
                .map_err(|e| format!("Failed to write config file: {}", e))?;

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
        };

        example.save()
    }
}
