#!/bin/bash
# Launch vis-grep (egui version)
# Force X11 backend for WSL
unset WAYLAND_DISPLAY
export WINIT_UNIX_BACKEND=x11
# Set RUST_LOG=debug for verbose logging, or RUST_LOG=info for less verbose
export RUST_LOG="${RUST_LOG:-info}"
./target/release/vis-grep "$@"
