# Idle Monitoring and Auto-Shutdown Feature

## Overview
Added auto-shutdown functionality to prevent the application from running indefinitely when forgotten.

## Features

### 1. Idle Monitoring
- Tracks user activity (mouse clicks, keyboard input)
- Configurable timeout period via config.yaml
- Grace period of 5 minutes after startup (allows remote login)
- Visual countdown in status bar

### 2. Terminal Auto-Close
- Terminal windows now auto-close when pager exits
- Works with less, more, mless, and other pagers
- No more leftover terminal windows

## Configuration

Add to your `config.yaml`:

```yaml
ui:
  auto_shutdown_minutes: 30  # Auto-shutdown after 30 minutes of inactivity
```

To disable auto-shutdown:
```yaml
ui:
  auto_shutdown_minutes: null  # Or just omit the line
```

## Status Bar Display
The idle timer status appears in the status bar:
- "Auto-shutdown in: 29m 45s" - Countdown to shutdown
- "Auto-shutdown: In grace period" - During 5-minute startup grace
- "Auto-shutdown: Idle timeout reached" - About to shutdown

## Testing
Use `test_idle.sh` to test with a 2-minute timeout:
```bash
./test_idle.sh
```

## How It Works
1. Any user interaction resets the idle timer
2. Timer only starts after 5-minute grace period
3. App closes gracefully when timeout is reached
4. Terminal commands include "; exit" to auto-close

## Use Cases
- Prevent resource usage when forgetting to close after work
- Safe for vacation/extended away periods
- Won't interrupt active usage (any interaction resets timer)
- Remote login protection via grace period