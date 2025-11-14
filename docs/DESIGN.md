# VisGrep Design Document

## Overview
This document outlines the refactoring strategy and feature priorities for VisGrep as it evolves from a monolithic structure to a more modular architecture.

## Current State
- Main.rs is ~1900 lines and growing
- Contains UI logic, state management, and mode switching
- Already has some modularization (grep_mode, tail_mode, preview, etc.)

## Refactoring Strategy

### Phase 1: Immediate Priorities
1. **Theme Support** (High Priority)
   - Add configuration for dark/light mode preference
   - Allow runtime theme switching in UI
   - Persist theme preference

2. **Begin UI Component Extraction**
   - Extract mode-specific UI rendering into dedicated modules
   - Create shared UI component modules (toolbars, panels, etc.)
   - Keep changes incremental to avoid breaking functionality

### Phase 2: Enhanced Functionality
1. **Filtering Capabilities**
   - Add filter input to tail mode preview pane
   - Implement file filtering in tail mode file list
   - Consider unified filter syntax across modes

2. **Layout Management**
   - Save/load tail layouts
   - Remember window positions and splits
   - User-defined layout presets

### Phase 3: Advanced Features
1. **File Management**
   - Drag and drop files onto tail mode
   - Dynamic file addition/removal in tail mode
   - File grouping and organization

2. **Performance Optimization**
   - Lazy loading of large files
   - Virtual scrolling for long outputs
   - Background threading improvements

## Proposed Module Structure

```
src/
├── main.rs (reduced to app setup and high-level coordination)
├── app/
│   ├── mod.rs
│   ├── state.rs (application state management)
│   └── mode.rs (mode switching logic)
├── ui/
│   ├── mod.rs
│   ├── theme.rs (theme management)
│   ├── toolbar.rs
│   ├── panels.rs
│   └── components/ (shared UI components)
├── modes/
│   ├── grep/
│   │   ├── mod.rs
│   │   ├── state.rs
│   │   └── ui.rs
│   └── tail/
│       ├── mod.rs
│       ├── state.rs
│       └── ui.rs
├── config/
│   ├── mod.rs
│   ├── theme.rs
│   └── persistence.rs
└── (existing modules: preview, search, input_handler, etc.)
```

## Implementation Guidelines

1. **Incremental Refactoring**
   - Each refactoring should be a small, working change
   - Maintain backward compatibility where possible
   - Test thoroughly after each change

2. **State Management**
   - Consider moving to a more formal state management pattern
   - Separate UI state from business logic
   - Make state changes predictable and testable

3. **Configuration**
   - Use a consistent configuration format (TOML/JSON)
   - Support both file-based and UI-based configuration
   - Provide sensible defaults

4. **Error Handling**
   - Implement consistent error handling across modules
   - Provide user-friendly error messages
   - Log errors appropriately

## Priority Order

1. Theme configuration (addresses immediate usability concern)
2. Basic UI component extraction (prevents further growth of main.rs)
3. Tail mode filtering (enhances current functionality)
4. Layout persistence (improves user experience)
5. Advanced features as needed

## Notes

- Keep performance as a primary concern throughout refactoring
- Maintain the current fast and responsive feel
- Consider accessibility in all UI changes
- Document new modules and APIs as they're created