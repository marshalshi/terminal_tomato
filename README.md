# Terminal Tomato

Terminal Tomato is a full-screen CLI tomato timer written in Rust. It provides a
simple TUI, configurable session lengths, optional MP3 sound playback, and daily
JSONL logs for tracking your working time.

## Features

- Full-screen terminal UI with large timer display
- Configurable work/short break/long break durations
- Pause, start, restart, cancel, and skip controls
- Optional MP3 sound on session completion
- Daily JSONL logs stored locally

## Quick Start

1. Copy and edit the config:

```bash
cp config.toml.example config.toml
```

2. Run the app:

```bash
cargo run
```

## Controls

- `s` start
- `p` pause/resume
- `r` restart current session
- `c` cancel current session
- `n` skip to next session
- `q` quit

## Configuration

Settings are loaded from `config.toml` in the repo root. The file is gitignored.

Example (`config.toml.example`):

```toml
work_minutes = 25
short_break_minutes = 5
long_break_minutes = 15
long_break_every = 4
sound_path = ""
log_dir = "logs"
```

Notes:

- `sound_path` should point to an MP3 file. Leave empty to disable sound.
- `log_dir` is a repo-local directory for JSONL logs.

## Logs

Each day is written to `logs/YYYY-MM-DD.jsonl`. Every line is a JSON object:

```json
{"start_ts":"2026-02-25T09:00:00+00:00","end_ts":"2026-02-25T09:25:00+00:00","session_type":"Work","duration_min":25,"status":"completed"}
```

## Build

```bash
cargo build --release
```

## Troubleshooting

- If audio doesn’t play, confirm `sound_path` is valid and readable.
- If the timer doesn’t start, press `s` to start the first session.
