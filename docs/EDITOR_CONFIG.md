# Editor Configuration

VisGrep now supports opening files directly in your preferred text editor from both grep and tail modes.

## Config File Location

The config file is located at:
- **Windows**: `%APPDATA%\vis-grep\config.yaml` (e.g., `C:\Users\YourName\AppData\Roaming\vis-grep\config.yaml`)
- **Linux/macOS**: `~/.config/vis-grep/config.yaml`

If the config file doesn't exist, VisGrep will create an example one on first run.

## Configuration

### Using config.yaml

Add an `editor` section to your config file:

```yaml
editor:
  command: "code"
  args: ["-g"]

# Other examples:
# Notepad++ on Windows
editor:
  command: "C:\\Program Files\\Notepad++\\notepad++.exe"
  args: []

# VSCode with line number
editor:
  command: "code"
  args: ["--goto"]

# Vim in a new terminal
editor:
  command: "gnome-terminal"
  args: ["--", "vim"]
```

### Using Environment Variables

If no editor is configured in `config.yaml`, VisGrep will check these environment variables in order:
1. `VISUAL` - Preferred visual editor
2. `EDITOR` - Default text editor

Example:
```bash
export VISUAL="code"
export EDITOR="vim"
```

### Fallback Editors

If no configuration is found, VisGrep will try these editors in order:

**Windows:**
- notepad++.exe
- notepad.exe

**Linux/macOS:**
- code (VSCode)
- vim
- nano
- gedit
- kate

## Usage

### In Tail Mode
When viewing a file in the preview pane, click the "📝 Editor" button next to the Explorer button to open the file in your editor.

### In Grep Mode
After performing a search, click the "📝 Editor" button in the toolbar to open the currently selected file.

## Advanced Configuration

### Opening at Specific Line (Future Enhancement)
Some editors support opening at a specific line number. This feature is planned for a future update where grep mode will open directly at the matched line.

### Multiple Editor Profiles
You might want different editors for different file types. This is also planned for a future enhancement.

## Troubleshooting

### Editor Not Opening
1. Check that the editor command is in your PATH
2. For Windows, use the full path to the executable
3. Check the log output for error messages
4. Try setting the EDITOR environment variable as a fallback

### Wrong Editor Opens
1. Check your config.yaml takes precedence over environment variables
2. Ensure the command path is correct
3. Clear any conflicting environment variables if needed