use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The main layout configuration for tail mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailLayout {
    pub name: String,
    pub version: u32,
    #[serde(default)]
    pub settings: LayoutSettings,
    #[serde(rename = "groups")]
    pub root_groups: Vec<FileGroup>,
}

/// Settings that apply to the entire layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutSettings {
    pub poll_interval_ms: Option<u64>,
    #[serde(default = "default_true")]
    pub auto_expand_active: bool,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            poll_interval_ms: None,
            auto_expand_active: true,
        }
    }
}

fn default_true() -> bool {
    true
}

/// A group that contains files and/or other groups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileGroup {
    #[serde(skip)]
    pub id: String, // Will be generated after loading
    pub name: String,
    pub icon: Option<String>,
    #[serde(skip)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub collapsed: bool,

    // Either files or subgroups (or both)
    #[serde(default)]
    pub files: Vec<FileEntry>,
    #[serde(default)]
    pub groups: Vec<FileGroup>,

    // Runtime state (not serialized)
    #[serde(skip)]
    pub has_activity: bool,
    #[serde(skip)]
    pub active_file_count: usize,
    #[serde(skip)]
    pub total_file_count: usize,
    #[serde(skip)]
    pub user_collapsed: Option<bool>, // Track if user manually collapsed/expanded
}

/// An individual file entry within a group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: Option<String>, // Display name override
    #[serde(default)]
    pub pattern: bool, // If true, path is a glob pattern
    #[serde(default)]
    pub paused: bool, // If true, file starts paused

    // Reference to actual TailedFile (set at runtime)
    #[serde(skip)]
    pub tailed_file_idx: Option<usize>,
}

impl TailLayout {
    /// Load a layout from a YAML file
    pub fn from_yaml_file(path: &PathBuf) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read layout file: {}", e))?;
        
        Self::from_yaml_str(&contents)
    }

    /// Parse layout from YAML string
    pub fn from_yaml_str(yaml: &str) -> Result<Self, String> {
        let mut layout: TailLayout = serde_yaml::from_str(yaml)
            .map_err(|e| format!("Failed to parse YAML: {}", e))?;
        
        // Assign IDs and parent references
        layout.assign_ids();
        
        Ok(layout)
    }

    /// Assign unique IDs to all groups and set up parent references
    fn assign_ids(&mut self) {
        let mut counter = 0;
        for group in &mut self.root_groups {
            Self::assign_group_ids(group, None, &mut counter);
        }
    }

    fn assign_group_ids(group: &mut FileGroup, parent_id: Option<String>, counter: &mut usize) {
        // Generate unique ID
        group.id = format!("group_{}", counter);
        *counter += 1;
        
        // Set parent reference
        group.parent_id = parent_id;
        
        // Count files in this group
        group.total_file_count = group.files.len();
        
        // Recursively process subgroups
        for subgroup in &mut group.groups {
            Self::assign_group_ids(subgroup, Some(group.id.clone()), counter);
            // Add subgroup's file count to parent
            group.total_file_count += subgroup.total_file_count;
        }
    }

    /// Find a group by ID
    pub fn find_group(&self, id: &str) -> Option<&FileGroup> {
        Self::find_group_in_list(&self.root_groups, id)
    }

    fn find_group_in_list<'a>(groups: &'a [FileGroup], id: &str) -> Option<&'a FileGroup> {
        for group in groups {
            if group.id == id {
                return Some(group);
            }
            if let Some(found) = Self::find_group_in_list(&group.groups, id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a mutable group by ID
    pub fn find_group_mut(&mut self, id: &str) -> Option<&mut FileGroup> {
        Self::find_group_mut_in_list(&mut self.root_groups, id)
    }

    fn find_group_mut_in_list<'a>(groups: &'a mut [FileGroup], id: &str) -> Option<&'a mut FileGroup> {
        for group in groups {
            if group.id == id {
                return Some(group);
            }
            if let Some(found) = Self::find_group_mut_in_list(&mut group.groups, id) {
                return Some(found);
            }
        }
        None
    }

    /// Get all file paths from the layout (flattened) with paused status
    pub fn get_all_file_paths(&self) -> Vec<(PathBuf, Option<String>, String, bool)> {
        let mut paths = Vec::new();
        for group in &self.root_groups {
            Self::collect_file_paths(group, &mut paths);
        }
        paths
    }

    fn collect_file_paths(group: &FileGroup, paths: &mut Vec<(PathBuf, Option<String>, String, bool)>) {
        // Add files from this group
        for file in &group.files {
            paths.push((file.path.clone(), file.name.clone(), group.id.clone(), file.paused));
        }
        
        // Recursively add files from subgroups
        for subgroup in &group.groups {
            Self::collect_file_paths(subgroup, paths);
        }
    }

    /// Update activity status for a group
    pub fn update_group_activity(&mut self, group_id: &str, child_active: bool) {
        // Get settings value before borrowing group
        let auto_expand = self.settings.auto_expand_active;
        
        // Extract values needed for propagation
        let (parent_id, activity_changed) = {
            if let Some(group) = self.find_group_mut(group_id) {
                // Update counts
                if child_active {
                    group.active_file_count += 1;
                } else {
                    group.active_file_count = group.active_file_count.saturating_sub(1);
                }
                
                // Update activity flag
                let was_active = group.has_activity;
                group.has_activity = group.active_file_count > 0;
                
                // Auto-expand if configured AND user hasn't manually collapsed
                if group.has_activity && auto_expand && group.user_collapsed.is_none() {
                    group.collapsed = false;
                }
                
                // If activity stops and user hasn't manually set state, allow collapse
                if !group.has_activity && group.user_collapsed.is_none() {
                    // Could auto-collapse here if desired, but for now just leave as is
                    // group.collapsed = true;
                }
                
                // Return parent info and whether activity changed
                (group.parent_id.clone(), was_active != group.has_activity)
            } else {
                (None, false)
            }
        };
        
        // Propagate to parent if needed
        if let Some(parent_id) = parent_id {
            if activity_changed {
                // Get the current activity status before recursive call
                let group_active = self.find_group(&group_id)
                    .map(|g| g.has_activity)
                    .unwrap_or(false);
                self.update_group_activity(&parent_id, group_active);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_layout() {
        let yaml = r#"
name: "Test Layout"
version: 1
groups:
  - name: "Web Services"
    files:
      - path: "/var/log/nginx/access.log"
        name: "Nginx Access"
      - path: "/var/log/nginx/error.log"
"#;
        
        let layout = TailLayout::from_yaml_str(yaml).unwrap();
        assert_eq!(layout.name, "Test Layout");
        assert_eq!(layout.version, 1);
        assert_eq!(layout.root_groups.len(), 1);
        assert_eq!(layout.root_groups[0].files.len(), 2);
    }

    #[test]
    fn test_nested_groups() {
        let yaml = r#"
name: "Nested Layout"
version: 1
groups:
  - name: "App"
    groups:
      - name: "Core"
        files:
          - path: "/app/core.log"
      - name: "Jobs"
        files:
          - path: "/app/jobs.log"
"#;
        
        let layout = TailLayout::from_yaml_str(yaml).unwrap();
        assert_eq!(layout.root_groups[0].groups.len(), 2);
        assert_eq!(layout.root_groups[0].total_file_count, 2);
    }
}