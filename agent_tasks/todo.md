# Tomato Timer CLI - Implementation Plan

- [x] Initialize Rust crate and baseline project structure
- [x] Add dependencies and config/log file setup
- [x] Implement config loader and defaults
- [x] Build TUI layout and input handling
- [x] Implement timer state machine and logging
- [x] Add audio playback and error handling
- [x] Add basic tests for config and log serialization
- [x] Review and document results

## Review

- Implemented full-screen TUI timer with pause/restart/cancel/skip controls
- Added repo-local config example and JSONL logging
- Added audio playback on session completion with graceful fallback
- Tests cover duration formatting, config validation, and log serialization

# Auto-Transfer Settings Plan

- [x] Add config fields for auto-start work and auto-start breaks
- [x] Update session transition logic to honor auto-start settings
- [x] Refresh config example and adjust tests for new config fields
- [x] Note behavior changes in review section

## Review

- Added config toggles to control auto-start for work and break sessions
- Session transitions now pause when auto-start is disabled, requiring manual start
- Updated config example and test data for new settings

# Show Seconds Setting Plan

- [x] Add config field to toggle seconds display (default on)
- [x] Update duration formatting to respect the setting
- [x] Refresh config example and adjust tests
- [x] Note behavior change in review section

## Review

- Added show_seconds config flag (default true) for minutes-only display
- Timer formatting now hides seconds when disabled
- Config example and tests updated for the new setting
