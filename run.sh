#!/bin/bash
# Launch vis-grep (egui version)
# Force X11 backend for WSL
unset WAYLAND_DISPLAY
export WINIT_UNIX_BACKEND=x11
export XCURSOR_SIZE=16

# Set RUST_LOG=debug for verbose logging, or RUST_LOG=info for less verbose
# Default to warn for normal usage (only warnings and errors)
export RUST_LOG="${RUST_LOG:-warn}"

# Use debug build when RUST_LOG=debug, otherwise use release
if [ "$RUST_LOG" = "debug" ]; then
    if [ -f target/debug/vis-grep ]; then
        ./target/debug/vis-grep "$@"
    else
        echo "Debug build not found. Run: cargo build"
        exit 1
    fi
else
    if [ -f target/release/vis-grep ]; then
        ./target/release/vis-grep "$@"
    else
        echo "Release build not found. Run: cargo build --release"
        exit 1
    fi
fi
