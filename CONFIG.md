# Configuration Guide

## Folder Presets

VisGrep supports configurable folder presets to quickly switch between commonly used search directories.

### Config File Location

The configuration file is located at:
- Linux/macOS: `~/.config/vis-grep/config.yaml`
- Windows: `%USERPROFILE%\.config\vis-grep\config.yaml`

### Example Configuration

Create or edit `~/.config/vis-grep/config.yaml`:

```yaml
folder_presets:
  - name: "Home"
    path: "~/"
  - name: "Current Directory"
    path: "."
  - name: "Logs"
    path: "~/logs"
  - name: "FIX Messages"
    path: "~/work/fix-logs"
  - name: "Nvim Config"
    path: "~/.config/nvim/lua/plugins"
  - name: "Dev Projects"
    path: "~/dev"
```

### Usage

1. Click the 📁 folder icon dropdown next to the Search Path field
2. Select a preset from the list
3. The path will be automatically expanded (including `~` for home directory)

### Features

- **Tilde expansion**: Paths starting with `~/` are automatically expanded to your home directory
- **Relative paths**: Use `.` for current directory
- **Custom names**: Give your presets meaningful names for easy identification
- **Quick access**: No need to type or browse for frequently used directories

### Notes

- If the config file doesn't exist, default presets (Home, Current Directory) will be used
- Changes to the config file require restarting VisGrep
- Invalid paths in config won't prevent the app from running
