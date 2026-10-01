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

# Clock Size Setting Plan

- [x] Add config field for clock size (large or small)
- [x] Render small clock text when configured
- [x] Refresh config example and add tests for size switch
- [x] Note behavior change in review section

## Review

- Added clock_size config with large/small options (default large)
- Small clock renders a single-line timer text
- Config example and tests updated for the new setting

# Work-Mode Background Music Plan

Assumption: the requested asset will be committed as repository-root `music.mp3`; it should loop only while a Work session is actively running (not while initially paused, manually paused, in either break type, or after cancel/skip/quit). The existing completion `sound_path`/`chime.mp3` behavior remains separate.

- [x] Add the original 24-second `music.mp3` background-audio asset and a backwards-compatible `music_path` setting (default `music.mp3`); document it in the example config and README.
- [x] Add a dedicated background-music controller that owns the Rodio output stream/sink, loops the configured MP3, and reports startup/decoder errors through the existing UI message path without crashing the timer.
- [x] Synchronize the controller from the timer state machine: start/resume music only for `Work + Running`; stop it on pause, restart-to-paused, cancel, skip/session advance, every break, and app exit; repeated ticks/transitions do not create duplicate sinks.
- [x] Preserve one-shot completion chimes and notifications, then add focused unit tests for desired music state across work/break and running/paused transitions plus config default behavior.
- [x] Run formatting and automated tests. Manual audio exercise was not run because this non-interactive environment cannot verify the active audio device.

## Verification

- `cargo fmt --check`, `cargo test`, and `git diff --check` pass.
- `ffprobe` confirms `music.mp3` is a decodable 24-second MP3.
- With `music.mp3` configured, the controller starts one looping sink for a running Work session and drops it for each non-running or non-Work state; completion `chime.mp3` remains independent.
- With the music setting missing, unreadable, or undecodable, the timer continues operating and surfaces a non-fatal message.

## Review

- Added an original, low-volume 24-second MP3 loop at `music.mp3`.
- Added `music_path`, defaulting to `music.mp3`; set it to an empty string to disable background music.
- Background audio is held by the app (rather than a detached thread), so it loops exactly while a Work timer is running and stops when that state changes.
- Existing completion sounds and notifications are unchanged.
- Review follow-up: startup failures are now retried on the next Work start/resume (not every UI tick), and `Decoder::new_looped` reopens the file at each loop rather than buffering an arbitrary track in memory.
