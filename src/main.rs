use chrono::{DateTime, Local};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{self, stdout, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Config {
    work_minutes: u64,
    short_break_minutes: u64,
    long_break_minutes: u64,
    long_break_every: u64,
    #[serde(default = "default_true")]
    auto_start_breaks: bool,
    #[serde(default = "default_true")]
    auto_start_work: bool,
    sound_path: String,
    log_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_every: 4,
            auto_start_breaks: true,
            auto_start_work: true,
            sound_path: String::new(),
            log_dir: "logs".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionType {
    Work,
    ShortBreak,
    LongBreak,
}

impl SessionType {
    fn label(self) -> &'static str {
        match self {
            SessionType::Work => "Work",
            SessionType::ShortBreak => "Short Break",
            SessionType::LongBreak => "Long Break",
        }
    }

    fn duration_minutes(self, config: &Config) -> u64 {
        match self {
            SessionType::Work => config.work_minutes,
            SessionType::ShortBreak => config.short_break_minutes,
            SessionType::LongBreak => config.long_break_minutes,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimerStatus {
    Running,
    Paused,
}

#[derive(Serialize)]
struct LogEntry {
    start_ts: String,
    end_ts: String,
    session_type: String,
    duration_min: u64,
    status: String,
}

struct App {
    config: Config,
    session_type: SessionType,
    status: TimerStatus,
    duration: Duration,
    remaining: Duration,
    last_tick: Instant,
    session_start: DateTime<Local>,
    completed_work_sessions: u64,
    message: Option<String>,
    should_exit: bool,
}

impl App {
    fn new(config: Config, message: Option<String>) -> Self {
        let duration = Duration::from_secs(config.work_minutes * 60);
        Self {
            config,
            session_type: SessionType::Work,
            status: TimerStatus::Paused,
            duration,
            remaining: duration,
            last_tick: Instant::now(),
            session_start: Local::now(),
            completed_work_sessions: 0,
            message,
            should_exit: false,
        }
    }

    fn on_tick(&mut self) {
        if self.status != TimerStatus::Running || self.should_exit {
            return;
        }

        let now = Instant::now();
        let delta = now.saturating_duration_since(self.last_tick);
        if delta >= self.remaining {
            self.remaining = Duration::from_secs(0);
            self.finish_current("completed", true, true);
        } else {
            self.remaining = self.remaining.saturating_sub(delta);
            self.last_tick = now;
        }
    }

    fn toggle_pause(&mut self) {
        match self.status {
            TimerStatus::Running => {
                self.status = TimerStatus::Paused;
            }
            TimerStatus::Paused => {
                if self.remaining == self.duration {
                    self.session_start = Local::now();
                }
                self.status = TimerStatus::Running;
                self.last_tick = Instant::now();
            }
        }
    }

    fn start(&mut self) {
        if self.status == TimerStatus::Paused {
            self.toggle_pause();
        }
    }

    fn restart(&mut self) {
        self.reset_current(false);
    }

    fn cancel(&mut self) {
        self.log_current("cancelled");
        self.reset_current(true);
    }

    fn start_new_work_session(&mut self) {
        self.set_session(SessionType::Work, true);
    }

    fn start_new_break_session(&mut self) {
        self.set_session(SessionType::ShortBreak, true);
    }

    fn skip(&mut self) {
        self.finish_current("skipped", true, false);
    }

    fn request_exit(&mut self) {
        if self.remaining < self.duration {
            self.log_current("cancelled");
        }
        self.should_exit = true;
    }

    fn finish_current(
        &mut self,
        status: &str,
        advance: bool,
        should_play_sound: bool,
    ) {
        self.log_current(status);
        if should_play_sound && status == "completed" {
            play_sound(&self.config.sound_path);
        }

        if advance {
            self.advance_session(status == "completed");
        } else {
            self.should_exit = true;
        }
    }

    fn advance_session(&mut self, completed: bool) {
        let next = match self.session_type {
            SessionType::Work => {
                if completed {
                    self.completed_work_sessions += 1;
                    if self.completed_work_sessions
                        % self.config.long_break_every
                        == 0
                    {
                        SessionType::LongBreak
                    } else {
                        SessionType::ShortBreak
                    }
                } else {
                    SessionType::ShortBreak
                }
            }
            SessionType::ShortBreak | SessionType::LongBreak => {
                SessionType::Work
            }
        };

        let auto_start = match next {
            SessionType::Work => self.config.auto_start_work,
            SessionType::ShortBreak | SessionType::LongBreak => {
                self.config.auto_start_breaks
            }
        };

        self.set_session(next, auto_start);
    }

    fn set_session(&mut self, session_type: SessionType, auto_start: bool) {
        self.session_type = session_type;
        let minutes = self.session_type.duration_minutes(&self.config);
        self.duration = Duration::from_secs(minutes * 60);
        self.remaining = self.duration;
        if auto_start {
            self.status = TimerStatus::Running;
            self.last_tick = Instant::now();
        } else {
            self.status = TimerStatus::Paused;
        }
        self.session_start = Local::now();
    }

    fn reset_current(&mut self, paused: bool) {
        let minutes = self.session_type.duration_minutes(&self.config);
        self.duration = Duration::from_secs(minutes * 60);
        self.remaining = self.duration;
        if paused {
            self.status = TimerStatus::Paused;
        } else {
            self.status = TimerStatus::Running;
            self.last_tick = Instant::now();
            self.session_start = Local::now();
        }
    }

    fn log_current(&mut self, status: &str) {
        if let Err(err) = write_log_entry(&self.config, self, status) {
            self.message = Some(format!("Log write failed: {err}"));
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (config, message) = load_config();
    let app = App::new(config, message);

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("{err}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(200);

    loop {
        terminal.draw(|frame| render_ui(frame, &app))?;

        if app.should_exit {
            return Ok(());
        }

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('s') => app.start(),
                    KeyCode::Char('p') => app.toggle_pause(),
                    KeyCode::Char('r') => app.restart(),
                    KeyCode::Char('c') => app.cancel(),
                    KeyCode::Char('w') => app.start_new_work_session(),
                    KeyCode::Char('b') => app.start_new_break_session(),
                    KeyCode::Char('n') => app.skip(),
                    KeyCode::Char('q') => app.request_exit(),
                    _ => {}
                }
            }
        }

        app.on_tick();
    }
}

fn render_ui(frame: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(frame.size());

    let status_color = match app.status {
        TimerStatus::Running => Color::Green,
        TimerStatus::Paused => Color::Yellow,
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            app.session_type.label(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            match app.status {
                TimerStatus::Running => "Running",
                TimerStatus::Paused => "Paused",
            },
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    let timer_text = format_duration(app.remaining);
    let timer_lines = big_timer_lines(&timer_text);
    let is_rest = matches!(
        app.session_type,
        SessionType::ShortBreak | SessionType::LongBreak
    );
    let timer_color = if is_rest {
        Color::LightGreen
    } else {
        Color::Yellow
    };
    let timer = Paragraph::new(timer_lines)
        .alignment(Alignment::Center)
        .style(
            Style::default()
                .fg(timer_color)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(timer_color)),
        );

    let message_text = app.message.as_deref().unwrap_or("Ready");
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            message_text,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::DIM),
        ),
        Span::raw("  "),
        Span::styled(
            "s: start  p: pause/resume  r: restart  c: cancel  w: work  b: break  n: next  q: quit",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::DIM),
        ),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(header, chunks[0]);
    frame.render_widget(timer, chunks[1]);
    frame.render_widget(footer, chunks[2]);
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let minutes = total / 60;
    let seconds = total % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

fn big_timer_lines(text: &str) -> Vec<Line<'static>> {
    let mut rows = vec![String::new(); 7];

    for (index, ch) in text.chars().enumerate() {
        let glyph = big_glyph(ch);
        for (row, part) in glyph.iter().enumerate() {
            if index > 0 {
                rows[row].push_str("  ");
            }
            rows[row].push_str(&thicken_row(part));
        }
    }

    let taller_rows = thicken_rows(&rows, 2);
    taller_rows.into_iter().map(Line::from).collect()
}

fn thicken_row(row: &str) -> String {
    let mut thick = String::with_capacity(row.len() * 2);
    for ch in row.chars() {
        if ch == ' ' {
            thick.push_str("  ");
        } else {
            thick.push_str("##");
        }
    }
    thick
}

fn thicken_rows(rows: &[String], scale: usize) -> Vec<String> {
    let mut expanded = Vec::with_capacity(rows.len() * scale);
    for row in rows {
        for _ in 0..scale {
            expanded.push(row.clone());
        }
    }
    expanded
}

fn big_glyph(ch: char) -> [&'static str; 7] {
    match ch {
        '0' => [
            " ##### ", "#     #", "#     #", "#     #", "#     #", "#     #",
            " ##### ",
        ],
        '1' => [
            "   #   ", "  ##   ", " # #   ", "   #   ", "   #   ", "   #   ",
            " ##### ",
        ],
        '2' => [
            " ##### ", "#     #", "      #", "  #### ", " #     ", "#      ",
            "#######",
        ],
        '3' => [
            " ##### ", "#     #", "      #", "  #### ", "      #", "#     #",
            " ##### ",
        ],
        '4' => [
            "#    # ", "#    # ", "#    # ", "#######", "     # ", "     # ",
            "     # ",
        ],
        '5' => [
            "#######", "#      ", "#      ", "###### ", "      #", "#     #",
            " ##### ",
        ],
        '6' => [
            " ##### ", "#     #", "#      ", "###### ", "#     #", "#     #",
            " ##### ",
        ],
        '7' => [
            "#######", "     # ", "    #  ", "   #   ", "  #    ", " #     ",
            "#      ",
        ],
        '8' => [
            " ##### ", "#     #", "#     #", " ##### ", "#     #", "#     #",
            " ##### ",
        ],
        '9' => [
            " ##### ", "#     #", "#     #", " ######", "      #", "#     #",
            " ##### ",
        ],
        ':' => [
            "       ", "   #   ", "       ", "       ", "   #   ", "       ",
            "       ",
        ],
        _ => [
            "       ", "       ", "       ", "       ", "       ", "       ",
            "       ",
        ],
    }
}

fn load_config() -> (Config, Option<String>) {
    let path = Path::new("config.toml");
    if !path.exists() {
        return (
            Config::default(),
            Some("config.toml not found; using defaults".to_string()),
        );
    }

    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(err) => {
            return (
                Config::default(),
                Some(format!("config.toml read error: {err}; using defaults")),
            );
        }
    };

    match toml::from_str::<Config>(&contents) {
        Ok(config) => validate_config(config),
        Err(err) => (
            Config::default(),
            Some(format!("config.toml parse error: {err}; using defaults")),
        ),
    }
}

fn validate_config(config: Config) -> (Config, Option<String>) {
    if config.work_minutes == 0
        || config.short_break_minutes == 0
        || config.long_break_minutes == 0
        || config.long_break_every == 0
        || config.log_dir.trim().is_empty()
    {
        return (
            Config::default(),
            Some("config.toml invalid; using defaults".to_string()),
        );
    }

    (config, None)
}

fn write_log_entry(config: &Config, app: &App, status: &str) -> io::Result<()> {
    let log_dir = PathBuf::from(&config.log_dir);
    fs::create_dir_all(&log_dir)?;

    let date = Local::now().format("%Y-%m-%d").to_string();
    let log_path = log_dir.join(format!("{date}.jsonl"));

    let entry = LogEntry {
        start_ts: app.session_start.to_rfc3339(),
        end_ts: Local::now().to_rfc3339(),
        session_type: app.session_type.label().to_string(),
        duration_min: app.duration.as_secs() / 60,
        status: status.to_string(),
    };

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;

    let line = serde_json::to_string(&entry)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn play_sound(sound_path: &str) {
    if sound_path.trim().is_empty() {
        return;
    }

    let path = Path::new(sound_path);
    if !path.exists() {
        return;
    }

    let path = path.to_path_buf();
    std::thread::spawn(move || {
        if let Ok((stream, handle)) = rodio::OutputStream::try_default() {
            if let Ok(file) = std::fs::File::open(&path) {
                if let Ok(source) =
                    rodio::Decoder::new(io::BufReader::new(file))
                {
                    let sink = rodio::Sink::try_new(&handle);
                    if let Ok(sink) = sink {
                        sink.append(source);
                        sink.sleep_until_end();
                        drop(stream);
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_minutes_seconds() {
        let duration = Duration::from_secs(125);
        assert_eq!(format_duration(duration), "02:05");
    }

    #[test]
    fn invalid_config_falls_back_to_defaults() {
        let config = Config {
            work_minutes: 0,
            short_break_minutes: 5,
            long_break_minutes: 15,
            long_break_every: 4,
            auto_start_breaks: true,
            auto_start_work: true,
            sound_path: String::new(),
            log_dir: "logs".to_string(),
        };

        let (validated, message) = validate_config(config);
        assert_eq!(validated.work_minutes, 25);
        assert!(message.is_some());
    }

    #[test]
    fn log_entry_serializes_to_json() {
        let entry = LogEntry {
            start_ts: "2026-02-25T09:00:00+00:00".to_string(),
            end_ts: "2026-02-25T09:25:00+00:00".to_string(),
            session_type: "Work".to_string(),
            duration_min: 25,
            status: "completed".to_string(),
        };

        let json = serde_json::to_string(&entry).expect("serialize log entry");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("parse json");
        assert_eq!(value["status"], "completed");
    }
}
