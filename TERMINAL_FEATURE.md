# Terminal with Pager Feature

vis-grep now supports opening files in a terminal with a pager (like `less` or `more`) for viewing log files.

## Usage

Click the **🖥️ Terminal** button in:
- Grep mode toolbar (next to Editor button)
- Tail mode toolbar (next to Editor button)  
- Tail mode tree view (small button next to each file)

## Configuration

Add a `terminal` section to your `config.yaml`:

### Windows Example
```yaml
terminal:
  command: "pwsh"
  args: ["-NoExit", "-Command"]
  pager: "more"
  pager_args: []
```

### Linux Example
```yaml
terminal:
  command: "gnome-terminal"
  args: ["--"]
  pager: "less"
  pager_args: ["-R"]  # -R enables color support
```

### macOS Example
```yaml
terminal:
  command: "Terminal"
  args: []
  pager: "less"
  pager_args: ["-R"]
```

## Fallback Behavior

If no terminal is configured, vis-grep will try common terminals:

**Windows:**
1. Windows Terminal (`wt`) with CMD and `more`
2. PowerShell Core (`pwsh`) with `Get-Content | more`
3. Command Prompt (`cmd`) with `more`

**Linux:**
1. GNOME Terminal with `less -R`
2. Konsole with `less -R`
3. Xfce Terminal with `less -R`
4. XTerm with `less -R`

**macOS:**
- Terminal.app via osascript with `less -R`

## Advanced Configuration Examples

### Windows Terminal with tabs
```yaml
terminal:
  command: "wt"
  args: ["-w", "0", "nt", "-d", ".", "cmd", "/k"]
  pager: "more"
  pager_args: []
```

### Linux with bat (syntax highlighting pager)
```yaml
terminal:
  command: "gnome-terminal"
  args: ["--"]
  pager: "bat"
  pager_args: ["--paging=always", "--style=numbers,changes"]
```

### PowerShell with custom viewer
```yaml
terminal:
  command: "pwsh"
  args: ["-NoExit", "-Command"]
  pager: "Get-Content"
  pager_args: ["-Tail", "1000", "-Wait", "|", "Out-Host", "-Paging"]
```

## Tips

1. **For log files**: Use pagers with follow mode:
   - Linux: `less +F` (press Ctrl+C to stop following)
   - PowerShell: `Get-Content -Wait`

2. **For color logs**: Enable color support:
   - `less -R` on Linux/macOS
   - `bat` for syntax highlighting

3. **For large files**: Consider pagers with search:
   - `less` with `/` for searching
   - `bat` with interactive mode