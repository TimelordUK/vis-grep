# Tail Mode Tree Layout Design

## Overview

When monitoring many log files (12+), a flat list becomes unwieldy. This feature adds support for organizing files into a collapsible tree structure via a layout configuration file.

## Use Cases

1. **Microservices**: Group logs by service
2. **Environment Separation**: Dev/Staging/Prod groupings
3. **Component Organization**: Frontend/Backend/Database logs
4. **Session Management**: Group related FIX session logs

## Design Principles

1. **Backwards Compatible**: `-f file1 file2` still works as a flat list
2. **Layout Files are Sessions**: Not part of main config, but loadable workspace definitions
3. **Visual Hierarchy**: Collapsible nodes with activity propagation
4. **Flexible Grouping**: Support nested groups and mixed files/groups

## Layout File Format

Using YAML for readability and ease of editing:

```yaml
# tail-layout.yaml
name: "Production Monitoring"
version: 1
settings:
  poll_interval_ms: 250
  auto_expand_active: true  # Auto-expand groups with activity

groups:
  - name: "Web Services"
    icon: "🌐"  # Optional emoji/icon
    collapsed: false  # Default state
    files:
      - path: "/var/log/nginx/access.log"
        name: "Nginx Access"  # Optional display name
      - path: "/var/log/nginx/error.log"
        name: "Nginx Errors"
      - path: "/var/log/apache2/access.log"
        name: "Apache Access"
    
  - name: "Application Logs"
    icon: "📱"
    groups:  # Nested groups
      - name: "Core Services"
        files:
          - path: "/app/logs/auth-service.log"
          - path: "/app/logs/user-service.log"
          - path: "/app/logs/payment-service.log"
      
      - name: "Background Jobs"
        collapsed: true
        files:
          - path: "/app/logs/scheduler.log"
          - path: "/app/logs/worker-1.log"
          - path: "/app/logs/worker-2.log"
  
  - name: "Trading Systems"
    icon: "📈"
    groups:
      - name: "FIX Sessions"
        files:
          - path: "/logs/fix_session_prod_*.log"
            pattern: true  # Glob pattern support
          - path: "/logs/fix_gateway.log"
      
      - name: "Market Data"
        files:
          - path: "/logs/market_data_feed.log"
          - path: "/logs/price_engine.log"

  - name: "Databases"
    icon: "🗄️"
    files:
      - path: "/var/log/postgresql/postgresql.log"
      - path: "/var/log/mysql/error.log"
      - path: "/var/log/redis/redis-server.log"
```

## Data Structures

```rust
#[derive(Debug, Clone)]
struct TailLayout {
    name: String,
    version: u32,
    settings: LayoutSettings,
    root_groups: Vec<FileGroup>,
}

#[derive(Debug, Clone)]
struct LayoutSettings {
    poll_interval_ms: Option<u64>,
    auto_expand_active: bool,
    // Future: default throttle settings, etc.
}

#[derive(Debug, Clone)]
struct FileGroup {
    id: Uuid,  // For UI state tracking
    name: String,
    icon: Option<String>,
    parent_id: Option<Uuid>,
    collapsed: bool,
    
    // Either files or subgroups (or both)
    files: Vec<FileEntry>,
    groups: Vec<FileGroup>,
    
    // Runtime state
    has_activity: bool,
    active_file_count: usize,
    total_file_count: usize,
}

#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    name: Option<String>,  // Display name override
    pattern: bool,  // If true, path is a glob pattern
    
    // Reference to actual TailedFile
    tailed_file_idx: Option<usize>,
}

// Extend existing TailedFile
impl TailedFile {
    // Add group membership
    group_id: Option<Uuid>,
}
```

## UI Mockup

```
Files Being Monitored:                    [+ Add] [📁 Load Layout] [⏸ Pause All]
┌─────────────────────────────────────────────────────────────────────────────┐
│ ▼ 🌐 Web Services (2 active / 3 total)                                      │
│   ● Nginx Access               2.4 KB  (+120 lines)  [⏸] [×]              │
│   ○ Nginx Errors              1.8 KB  (idle)         [⏸] [×]              │
│   ● Apache Access             3.1 KB  (+45 lines)    [⏸] [×]              │
│                                                                              │
│ ▼ 📱 Application Logs (3 active / 6 total)                                  │
│   ▼ Core Services (2/3)                                                     │
│     ● auth-service.log        5.2 KB  (+89 lines)    [⏸] [×]              │
│     ● user-service.log        4.7 KB  (+67 lines)    [⏸] [×]              │
│     ○ payment-service.log     2.1 KB  (idle)         [⏸] [×]              │
│   ▶ Background Jobs (1/3)                                                   │
│                                                                              │
│ ▶ 📈 Trading Systems (0/5)                                                  │
│ ▶ 🗄️ Databases (0/3)                                                        │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Command Line Usage

```bash
# Load a layout file
vis-grep --tail-layout production.yaml

# Or shorthand
vis-grep -tl production.yaml

# Can still use traditional mode
vis-grep -f file1.log file2.log

# Mix: load layout and add extra files
vis-grep --tail-layout base.yaml -f extra.log
```

## Implementation Plan

### Phase 1: Core Tree Structure
1. ✅ Design document (this file)
2. Add layout file parsing (serde + YAML)
3. Extend TailedFile with group membership
4. Create FileGroup tree structure
5. Add tree traversal utilities

### Phase 2: UI Implementation
1. Replace flat file list with tree widget
2. Implement expand/collapse with state persistence
3. Add group-level controls (pause group, remove group)
4. Show activity indicators at group level
5. Activity propagation (child → parent)

### Phase 3: Layout Management
1. Load layout from file command
2. Save current state as layout
3. Layout file validation
4. Recent layouts menu

### Phase 4: Advanced Features
1. Glob pattern support for dynamic file discovery
2. Group-level throttling settings
3. Group filtering (show only active groups)
4. Drag & drop file organization
5. Layout templates/presets

## Activity Propagation

When a file has activity:
1. Set file's `is_active = true`
2. Walk up the tree, setting each parent group's `has_activity = true`
3. Update group's `active_file_count`
4. Optionally auto-expand groups with activity

```rust
fn propagate_activity(&mut self, file_id: usize, active: bool) {
    if let Some(group_id) = self.files[file_id].group_id {
        self.update_group_activity(group_id, active);
    }
}

fn update_group_activity(&mut self, group_id: Uuid, child_active: bool) {
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
        
        // Auto-expand if configured
        if group.has_activity && self.settings.auto_expand_active {
            group.collapsed = false;
        }
        
        // Propagate to parent
        if let Some(parent_id) = group.parent_id {
            if was_active != group.has_activity {
                self.update_group_activity(parent_id, group.has_activity);
            }
        }
    }
}
```

## Rendering Logic

```rust
fn render_file_tree(&mut self, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .max_height(400.0)  // Larger for tree view
        .show(ui, |ui| {
            for group in &mut self.layout.root_groups {
                self.render_group(ui, group, 0);
            }
            
            // Ungrouped files at the end
            for (idx, file) in self.tail_state.files.iter_mut().enumerate() {
                if file.group_id.is_none() {
                    self.render_file_entry(ui, file, idx, 0);
                }
            }
        });
}

fn render_group(&mut self, ui: &mut egui::Ui, group: &mut FileGroup, depth: usize) {
    let indent = depth * 20.0;
    
    ui.horizontal(|ui| {
        ui.add_space(indent);
        
        // Expand/collapse arrow
        let arrow = if group.collapsed { "▶" } else { "▼" };
        if ui.small_button(arrow).clicked() {
            group.collapsed = !group.collapsed;
        }
        
        // Group icon
        if let Some(icon) = &group.icon {
            ui.label(icon);
        }
        
        // Group name with activity count
        let label = format!("{} ({} active / {} total)", 
            group.name, 
            group.active_file_count, 
            group.total_file_count
        );
        
        let color = if group.has_activity {
            egui::Color32::from_rgb(200, 255, 200)  // Light green
        } else {
            ui.style().visuals.text_color()
        };
        
        ui.colored_label(color, label);
        
        // Group controls
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("⏸").on_hover_text("Pause group").clicked() {
                self.pause_group(group.id);
            }
            if ui.small_button("×").on_hover_text("Remove group").clicked() {
                self.mark_group_for_removal(group.id);
            }
        });
    });
    
    // Render children if expanded
    if !group.collapsed {
        // Render subgroups
        for subgroup in &mut group.groups {
            self.render_group(ui, subgroup, depth + 1);
        }
        
        // Render files
        for file_entry in &group.files {
            if let Some(file_idx) = file_entry.tailed_file_idx {
                if let Some(file) = self.tail_state.files.get_mut(file_idx) {
                    self.render_file_entry(ui, file, file_idx, depth + 1);
                }
            }
        }
    }
}
```

## Configuration Storage

Layout files are stored separately from the main config:

```
~/.config/vis-grep/
  ├── config.yaml          # Main app config
  └── layouts/             # Layout files
      ├── default.yaml
      ├── production.yaml
      └── development.yaml
```

## Benefits

1. **Scalability**: Can handle 50+ files organized in logical groups
2. **Context**: Groups provide semantic meaning to sets of files
3. **Efficiency**: Can pause/remove entire groups at once
4. **Discoverability**: Easy to see which subsystems are active
5. **Reusability**: Save and share layout configurations

## Future Enhancements

1. **Smart Grouping**: Auto-detect common patterns and suggest groups
2. **Group Templates**: Predefined layouts for common scenarios
3. **Activity Heatmap**: Visual representation of group activity over time
4. **Group-level Filtering**: Apply filters to specific groups only
5. **Layout Inheritance**: Base layouts that can be extended