# Configuration Guide

VisGrep supports two types of configuration: folder presets and saved search patterns.

## Folder Presets

Quickly switch between commonly used search directories.

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

## Saved Search Patterns

Save complex regex patterns with friendly names for quick recall.

### Configuration

Add to your `~/.config/vis-grep/config.yaml`:

```yaml
saved_patterns:
  # FIX Message Patterns
  - name: "Execution Report"
    pattern: "35=8"
    description: "MsgType = Execution Report"
    category: "FIX - MsgType"

  - name: "Manual Fill in ExecReport"
    pattern: "35=8.*71=2"
    description: "Find Manual orders (71=2) in Execution Reports"
    category: "FIX - Combined"

  # Error Patterns
  - name: "Error"
    pattern: "(?i)error"
    description: "Case-insensitive error messages"
    category: "Errors"

  # Generic Patterns
  - name: "IP Address"
    pattern: "\\b(?:[0-9]{1,3}\\.){3}[0-9]{1,3}\\b"
    description: "IPv4 addresses"
    category: "Generic"
```

### Fields

- **name**: Display name in the dropdown (required)
- **pattern**: The regex pattern to search for (required)
- **description**: Shown as tooltip on hover (optional)
- **category**: Groups patterns in dropdown (optional)

### Usage

1. Click the 📝 pencil icon dropdown next to the Search Query field
2. Patterns are organized by category
3. Hover over a pattern to see its description
4. Click a pattern to load it into the search field
5. Press Search or Enter to execute

### Tips

- Use categories to organize patterns: "FIX - MsgType", "FIX - Status", "Errors", etc.
- Complex patterns with special characters need proper escaping:
  - Use `\\b` for word boundaries
  - Use `(?i)` for case-insensitive matching
  - Use `.*` to match anything in between
- Patterns from your React GUI can be copied directly
- Share your config file with team members for consistency

### Example FIX Patterns

Common FIX message search patterns:

```yaml
saved_patterns:
  - name: "Order Status Held"
    pattern: "150=H"
    description: "ExecType = Order Status (150=H)"
    category: "FIX - Status"

  - name: "Order Rejected"
    pattern: "150=8"
    description: "ExecType = Rejected (150=8)"
    category: "FIX - Status"

  - name: "Order Filled"
    pattern: "150=F"
    description: "ExecType = Trade/Fill (150=F)"
    category: "FIX - Status"

  - name: "Reject in ExecReport or Ack"
    pattern: "(35=8|35=J).*150=8"
    description: "Rejections (150=8) in exec reports or acks"
    category: "FIX - Combined"
```

### Notes

- Patterns are loaded on application startup
- No limit on number of patterns
- Empty categories default to "Other"
- Patterns apply to the current search mode (regex/literal)
