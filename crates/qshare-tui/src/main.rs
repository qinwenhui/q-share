//! q-share terminal UI (ratatui) — keyboard-driven dashboard.
//!
//! Two modes:
//!   - **Command mode** (default): `s` start · `x` stop · `q` / `Ctrl-C` quit,
//!     `Tab` to start editing the settings fields.
//!   - **Edit mode**: type into the focused field (`root` / `port` / `bind`),
//!     `↑`/`↓` (or `Tab` / `Shift+Tab`) to move, `Enter` or `Esc` to return to
//!     command mode.
//!
//! While editing, letters are typed as-is — `q`/`s`/`x` are never commands,
//! so paths containing them can be entered safely. `Ctrl+C` always quits.
//!
//! While the server runs, a background task polls `/api/stats` and `/api/log`
//! every second, so the counters and the bottom panel stay live.
//!
//! Layout:
//!   ┌─ header ─────────────────────────────────────────────────────────┐
//!   │  q-share · status · URL · up-time · mode                        │
//!   ├─ settings (editable) ──────────┬─ live panel ──────────────────┤
//!   │  root: <path>                  │  ┌─ QR (unicode) ─┐ stats      │
//!   │  port: <n>   bind: <addr>      │  │                │ conns      │
//!   │  <mode-specific hints>         │  │                │ bytes      │
//!   └────────────────────────────────┴──────────────────────────────┘
//!   log: last N events

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use parking_lot::Mutex;
use qshare_core::{Server, ServerConfig, ServerHandle, StatsSnapshot};
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::{Frame, Terminal};

#[derive(Parser, Debug)]
#[command(name = "qshare-tui", version, about = "q-share terminal UI")]
struct Args {
    /// Initial directory to share (default: current dir).
    /// Can also be passed positionally: `qshare-tui .`
    root: Option<PathBuf>,
    /// Directory to share (overrides positional).
    #[arg(long = "root", short = 'r', value_name = "ROOT")]
    root_flag: Option<PathBuf>,
    #[arg(long, short = 'p', default_value_t = 8888)]
    port: u16,
    #[arg(long, short = 'b', default_value = "0.0.0.0")]
    bind: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Focus {
    Root,
    Port,
    Bind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Idle,
    Starting,
    Running,
    Failed,
}

/// Live dashboard data — written by the background poller, read by the UI
/// thread each frame.
#[derive(Default, Clone)]
struct LiveData {
    stats: StatsSnapshot,
    log: Vec<String>,
}

impl LiveData {
    /// Append a line, keeping the ring bounded (matches the 200-line cap the
    /// UI panel can actually display).
    fn push_log(&mut self, line: String) {
        self.log.push(line);
        if self.log.len() > 200 {
            let drop = self.log.len() - 200;
            self.log.drain(..drop);
        }
    }
}

/// One line from `/api/log`.
#[derive(serde::Deserialize, Clone)]
struct LogLine {
    ts_ms: u64,
    level: String,
    msg: String,
}

#[derive(serde::Deserialize)]
struct LogResponse {
    lines: Vec<LogLine>,
}

struct State {
    root: String,
    port: String,
    bind: String,
    /// `Some(field)` = editing that field; `None` = command mode.
    focus: Option<Focus>,
    status: Status,
    url: Option<String>,
    started: Option<Instant>,
    qr: Option<Vec<Vec<bool>>>, // true = black module
    qr_modules: u32,            // edge size (modules per side)
    live: Arc<Mutex<LiveData>>,
    runtime: Option<tokio::runtime::Runtime>,
    server: Option<ServerHandle>,
    /// Animation frame for the pre-start campfire. Ticks at ~6 FPS so the
    /// flame appears to flicker without burning CPU.
    flame_frame: u32,
}

impl State {
    fn new(args: Args) -> Self {
        let root = args
            .root_flag
            .or(args.root)
            .or_else(|| std::env::current_dir().ok())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut live = LiveData::default();
        live.push_log("ready · s start · Tab edit · q quit".into());
        Self {
            root,
            port: args.port.to_string(),
            bind: args.bind,
            focus: None,
            status: Status::Idle,
            url: None,
            started: None,
            qr: None,
            qr_modules: 0,
            live: Arc::new(Mutex::new(live)),
            runtime: None,
            server: None,
            flame_frame: 0,
        }
    }

    fn log(&mut self, line: impl Into<String>) {
        let now = chrono_like_now();
        self.live
            .lock()
            .push_log(format!("[{now}] {}", line.into()));
    }

    fn start(&mut self) {
        let root = PathBuf::from(self.root.trim());
        if root.as_os_str().is_empty() || !root.is_dir() {
            self.log("start failed: invalid root directory");
            self.status = Status::Failed;
            return;
        }
        let port: u16 = match self.port.trim().parse() {
            Ok(p) => p,
            Err(_) => {
                self.log("start failed: bad port");
                self.status = Status::Failed;
                return;
            }
        };
        let host: IpAddr = match self.bind.trim().parse() {
            Ok(h) => h,
            Err(_) => {
                self.log("start failed: invalid bind address");
                self.status = Status::Failed;
                return;
            }
        };
        let cfg = ServerConfig {
            root,
            host,
            port,
            ..Default::default()
        };

        // Spin up a runtime that owns the server.
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                self.log(format!("runtime: {e}"));
                self.status = Status::Failed;
                return;
            }
        };

        self.status = Status::Starting;
        let res: anyhow::Result<(String, Vec<Vec<bool>>, u32, ServerHandle)> =
            runtime.block_on(async move {
                let server = Server::new(cfg.clone())?;
                let handle = server.start().await?;
                let addr = handle.local_addr();
                let host = match addr.ip().to_string().as_str() {
                    "0.0.0.0" | "::" => local_ip().unwrap_or_else(|| "127.0.0.1".into()),
                    h => h.to_string(),
                };
                let url = format!("http://{host}:{}", addr.port());
                let code = qrcode::QrCode::new(url.as_bytes())?;
                let modules: u32 = code.width() as u32;
                let mut grid: Vec<Vec<bool>> = code
                    .to_colors()
                    .chunks(modules as usize)
                    .map(|row| row.iter().map(|c| *c == qrcode::Color::Dark).collect())
                    .collect();
                // qrcode returns top row first; we use as-is.
                let _ = &mut grid;
                Ok((url, grid, modules, handle))
            });

        match res {
            Ok((url, grid, modules, handle)) => {
                self.url = Some(url.clone());
                self.qr = Some(grid);
                self.qr_modules = modules;
                let port = handle.local_addr().port();
                self.server = Some(handle);
                self.runtime = Some(runtime);
                self.status = Status::Running;
                self.started = Some(Instant::now());
                // Fresh server → fresh counters. The poller below refreshes
                // them on its first tick.
                self.live.lock().stats = StatsSnapshot::default();
                let poll_host = self
                    .bind
                    .trim()
                    .parse::<IpAddr>()
                    .map(|ip| {
                        if ip.is_unspecified() {
                            match ip {
                                IpAddr::V6(_) => IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
                                _ => IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                            }
                        } else {
                            ip
                        }
                    })
                    .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
                let live = Arc::clone(&self.live);
                if let Some(rt) = &self.runtime {
                    rt.spawn(poll_loop(format!("http://{poll_host}:{port}"), live));
                }
                self.log(format!("running · {url}"));
            }
            Err(e) => {
                self.status = Status::Failed;
                self.log(format!("start failed: {e}"));
            }
        }
    }

    fn stop(&mut self) {
        if let (Some(rt), Some(handle)) = (self.runtime.as_ref(), self.server.take()) {
            rt.block_on(handle.shutdown());
        }
        self.runtime = None;
        self.url = None;
        self.qr = None;
        self.qr_modules = 0;
        self.started = None;
        self.status = Status::Idle;
        self.log("stopped");
    }
}

fn chrono_like_now() -> String {
    // Avoid pulling chrono: HH:MM:SS from system time.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let s = now % 86400;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    format!("{h:02}:{m:02}:{sec:02}")
}

fn local_ip() -> Option<String> {
    use std::net::UdpSocket;
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    sock.connect("8.8.8.8:80").ok()?;
    Some(sock.local_addr().ok()?.ip().to_string())
}

fn view(state: &State, frame: &mut Frame) {
    let area = frame.area();
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(10),   // body
            Constraint::Length(6), // log
        ])
        .split(area);

    // ── Header ────────────────────────────────────────────────────────────
    let (status_label, status_color) = match state.status {
        Status::Idle => ("● idle", Color::DarkGray),
        Status::Starting => ("● starting", Color::Yellow),
        Status::Running => ("● running", Color::Green),
        Status::Failed => ("● failed", Color::Red),
    };
    let uptime = state
        .started
        .map(|t| format!("up {}", human_duration(t.elapsed())))
        .unwrap_or_default();
    let url = state.url.clone().unwrap_or_else(|| "—".into());
    let mode_tag = match state.focus {
        None => "cmd",
        Some(Focus::Root) => "edit:root",
        Some(Focus::Port) => "edit:port",
        Some(Focus::Bind) => "edit:bind",
    };
    let header_text = Line::from(vec![
        Span::styled(
            " q-share ",
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            status_label,
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(url, Style::default().fg(Color::White)),
        Span::raw("  "),
        Span::styled(uptime, Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled(mode_tag, Style::default().fg(Color::Cyan)),
    ]);
    frame.render_widget(
        Paragraph::new(header_text).block(Block::default().borders(Borders::BOTTOM)),
        outer[0],
    );

    // ── Body (settings | live) ────────────────────────────────────────────
    let body_h = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(38), Constraint::Min(30)])
        .split(outer[1]);

    let focus = state.focus;
    let mut settings_lines = vec![];
    for (label, value, is_focused) in [
        ("root", state.root.as_str(), focus == Some(Focus::Root)),
        ("port", state.port.as_str(), focus == Some(Focus::Port)),
        ("bind", state.bind.as_str(), focus == Some(Focus::Bind)),
    ] {
        let prefix = if is_focused { "▸ " } else { "  " };
        let style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        settings_lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(format!("{label:<5} "), style),
            Span::raw(value),
        ]));
    }
    settings_lines.push(Line::raw(""));
    let hint: Vec<Span> = match focus {
        None => vec![
            Span::styled(" s ", Style::default().bg(Color::Green).fg(Color::Black)),
            Span::raw(" start  "),
            Span::styled(" x ", Style::default().bg(Color::Red).fg(Color::Black)),
            Span::raw(" stop   "),
            Span::styled(" q ", Style::default().bg(Color::DarkGray).fg(Color::White)),
            Span::raw(" quit  "),
            Span::styled(" Tab ", Style::default().bg(Color::Blue).fg(Color::Black)),
            Span::raw(" edit"),
        ],
        Some(f) => vec![
            Span::raw("editing "),
            Span::styled(
                format!("{f:?}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" · "),
            Span::styled(" ↑/↓ ", Style::default().bg(Color::Blue).fg(Color::Black)),
            Span::raw(" move · "),
            Span::styled(" Enter ", Style::default().bg(Color::Blue).fg(Color::Black)),
            Span::raw("/"),
            Span::styled(" Esc ", Style::default().bg(Color::Blue).fg(Color::Black)),
            Span::raw(" done"),
        ],
    };
    settings_lines.push(Line::from(hint));
    let settings = Paragraph::new(settings_lines).block(
        Block::default()
            .title(Span::styled(" settings ", Style::default().fg(Color::Cyan)))
            .borders(Borders::ALL),
    );
    frame.render_widget(settings, body_h[0]);

    // Live panel: QR (left) + stats (right)
    let live = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(36), Constraint::Min(20)])
        .split(body_h[1]);

    let qr_area = live[0];
    if let Some(grid) = &state.qr {
        render_qr(frame, qr_area, grid);
    } else {
        render_campfire(frame, qr_area, state.flame_frame);
    }

    let dash = state.live.lock().clone();
    let stats_lines = vec![
        Line::from(format!("active  {}", dash.stats.active)),
        Line::from(format!("bytes   {}", human_bytes(dash.stats.bytes_served))),
        Line::from(format!("errors  {}", dash.stats.errors)),
    ];
    let stats_widget = Paragraph::new(stats_lines).block(
        Block::default()
            .title(Span::styled(" stats ", Style::default().fg(Color::Cyan)))
            .borders(Borders::ALL),
    );
    frame.render_widget(stats_widget, live[1]);

    // ── Log ───────────────────────────────────────────────────────────────
    let log_items: Vec<ListItem> = dash
        .log
        .iter()
        .rev()
        .take(5)
        .map(|s| ListItem::new(s.as_str()))
        .collect();
    let log_widget = List::new(log_items)
        .block(
            Block::default()
                .title(Span::styled(" log ", Style::default().fg(Color::Cyan)))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(log_widget, outer[2]);
}

fn render_qr(frame: &mut Frame, area: Rect, grid: &[Vec<bool>]) {
    // Render using half-block characters so each cell encodes two modules.
    let n = grid.len() as u32;
    if n == 0 {
        return;
    }
    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    // Leave a 2-cell margin around the QR (quiet zone).
    let avail_w = (inner.width.saturating_sub(2)).max(1) as u32;
    let avail_h = inner.height.saturating_sub(1).max(1) as u32 * 2; // each row = 2 modules

    fn ceil_div(a: u32, b: u32) -> u32 {
        a.div_ceil(b)
    }
    let scale_w = ceil_div(n, avail_w);
    let scale_h = ceil_div(n, avail_h);
    let scale = scale_w.max(scale_h).max(1);
    let cell_w = (n.div_ceil(scale)) as u16;

    let x_off = if inner.width > cell_w {
        (inner.width - cell_w) / 2
    } else {
        0
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut y = 0u32;
    while y < n {
        let mut spans: Vec<Span> = Vec::new();
        if x_off > 0 {
            spans.push(Span::raw(" ".repeat(x_off as usize)));
        }
        let mut x = 0u32;
        while x < n {
            let top = sample(grid, x, y);
            let bot = sample(grid, x, y.saturating_add(scale));
            let ch = match (top, bot) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            };
            spans.push(Span::styled(ch.to_string(), Style::default()));
            x = x.saturating_add(scale);
        }
        lines.push(Line::from(spans));
        y = y.saturating_add(2 * scale);
    }

    let para = Paragraph::new(lines).block(
        Block::default()
            .title(Span::styled(" QR ", Style::default().fg(Color::Cyan)))
            .borders(Borders::ALL),
    );
    frame.render_widget(para, area);
}

fn sample(grid: &[Vec<bool>], x: u32, y: u32) -> bool {
    let xi = (x as usize).min(grid[0].len().saturating_sub(1));
    let yi = (y as usize).min(grid.len().saturating_sub(1));
    grid[yi][xi]
}

fn human_duration(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{s}s")
    } else if s < 3600 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{}h{:02}m", s / 3600, (s % 3600) / 60)
    }
}

fn human_bytes(n: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{} {}", n, UNITS[0])
    } else {
        format!("{:.1} {}", v, UNITS[i])
    }
}

// ─── Animated pixel campfire ────────────────────────────────────────────────
//
// Four flame poses, each 16 cols × 8 rows. Painted with half-block
// characters so the inner colour and the outer glow are emitted in the
// same character cell, doubling the vertical resolution without taking
// more screen space than a single line per row.
//
// Encoding: '.' = empty, 'D' = dark stone (top half), 'S' = stone (bottom
// half), 'L' = log, 'r' = red top / 'R' = red bottom, 'o' = orange top /
// 'O' = orange bottom, 'y' = yellow top / 'Y' = yellow bottom. The top
// half goes into the foreground colour of the upper half-block; the
// bottom half into the foreground of the lower half-block. Mixing them
// in one row gives a vertical "flame above ember" look in two lines.

/// 16-wide × 8-row flame poses. Each row is two lines tall (top/bottom
/// halves of one screen row). Four frames cycle to read as a flicker.
const FLAME_FRAMES: [[&str; 16]; 4] = [
    // Frame 0 — neutral, slight right lean
    [
        ".......yY......",
        ".......yY......",
        ".....yyyyYY....",
        ".....yyyyYY....",
        "....oooyyYYY...",
        "....oooyyYYY...",
        "...Rroooooooo..",
        "...Rroooooooo..",
        "...RRroooooOO..",
        "...RRroooooOO..",
        ".....RRrooo....",
        ".....RRrooo....",
        ".....LLLLLL....",
        ".....LLLLLL....",
        "..DSSSSSSSSD...",
        "..DSSSSSSSSD...",
    ],
    // Frame 1 — lean left
    [
        "......yY.......",
        "......yY.......",
        ".....yYYy.....",
        ".....yYYy.....",
        "....oOoyYY....",
        "....oOoyYY....",
        "...Rroooyy....",
        "...Rroooyy....",
        "..RRrooooo....",
        "..RRrooooo....",
        "...RRRooo.....",
        "...RRRooo.....",
        ".....LLLLLL....",
        ".....LLLLLL....",
        "..DSSSSSSSSD...",
        "..DSSSSSSSSD...",
    ],
    // Frame 2 — tall reach
    [
        ".......yY......",
        ".......yY......",
        "......yyyY.....",
        "......yyyY.....",
        ".....yyyYYY....",
        ".....yyyYYY....",
        "....oooyyYYy...",
        "....oooyyYYy...",
        "...RroooooYY...",
        "...RroooooYY...",
        "...RRrooooOO...",
        "...RRrooooOO...",
        ".....LLLLLL....",
        ".....LLLLLL....",
        "..DSSSSSSSSD...",
        "..DSSSSSSSSD...",
    ],
    // Frame 3 — lean right
    [
        ".........yY....",
        ".........yY....",
        "........yyyYY..",
        "........yyyYY..",
        "........yYOOo..",
        "........yYOOo..",
        ".......yYOOooR.",
        ".......yYOOooR.",
        ".......OOOOooRR",
        ".......OOOOooRR",
        "........OOORR..",
        "........OOORR..",
        ".....LLLLLL....",
        ".....LLLLLL....",
        "..DSSSSSSSSD...",
        "..DSSSSSSSSD...",
    ],
];

/// 16-frame line list. Index 0 is the upper half of screen row 0, index
/// 1 is the lower half of row 0, index 2 is the upper half of row 1, etc.
fn flame_lines(frame: usize) -> [&'static str; 16] {
    FLAME_FRAMES[frame % FLAME_FRAMES.len()]
}

/// Render the animated campfire into the given rect, centred. Returns
/// the number of actual character rows consumed (always 8 here, but we
/// don't rely on it — the caller pads with blank space below).
fn render_campfire(frame: &mut Frame, area: Rect, frame_idx: u32) {
    let lines = flame_lines(frame_idx as usize);
    let style_for = |c: char| -> Style {
        // Top/bottom glyphs share colour with their halves: lowercase
        // (top) and uppercase (bottom) of the same letter carry the same
        // semantic colour so we can lay down two adjacent lines that look
        // like a single row.
        let base = match c {
            'y' | 'Y' => Color::Yellow,
            'o' | 'O' => Color::LightRed,
            'r' | 'R' => Color::Red,
            'L' => Color::Rgb(180, 100, 40), // warm brown log
            'S' => Color::DarkGray,
            'D' => Color::Black,
            _ => Color::Reset,
        };
        Style::default().fg(base)
    };

    // Two physical screen rows per "pixel row" of the art. We loop 8 times
    // and render two lines (top half then bottom half) for each.
    let mut para_lines: Vec<Line> = Vec::with_capacity(8);
    for row in 0..8 {
        let top = lines[row * 2];
        let bot = lines[row * 2 + 1];
        let mut top_spans: Vec<Span> = Vec::new();
        let mut bot_spans: Vec<Span> = Vec::new();
        for (t, b) in top.chars().zip(bot.chars()) {
            top_spans.push(Span::styled("▀".to_string(), style_for(t)));
            bot_spans.push(Span::styled("▄".to_string(), style_for(b)));
        }
        para_lines.push(Line::from(top_spans));
        para_lines.push(Line::from(bot_spans));
    }

    let inner = area.inner(Margin {
        vertical: 1,
        horizontal: 1,
    });
    let art_w = 16u16;
    let x_off = inner.width.saturating_sub(art_w) / 2;
    let prefix: String = " ".repeat(x_off as usize);
    let centred: Vec<Line> = para_lines
        .into_iter()
        .map(|l| {
            let mut v = vec![Span::raw(prefix.clone())];
            v.extend(l.spans);
            Line::from(v)
        })
        .collect();

    let mut final_lines: Vec<Line> = centred;
    final_lines.push(Line::from(""));
    let caption_pad: String = " ".repeat(x_off as usize);
    final_lines.push(Line::from(vec![
        Span::raw(caption_pad),
        Span::styled("press s to ignite", Style::default().fg(Color::DarkGray)),
    ]));

    let block = Block::default()
        .title(Span::styled(
            " flame ",
            Style::default().fg(Color::LightYellow),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let para = Paragraph::new(final_lines).block(block);
    frame.render_widget(para, area);
}

/// Result of handling one key press.
#[derive(Debug, PartialEq)]
enum KeyAction {
    /// The key was handled.
    Consumed,
    /// The app should exit.
    Quit,
    /// Nothing to do.
    Ignored,
}

fn next_focus(f: Focus) -> Focus {
    match f {
        Focus::Root => Focus::Port,
        Focus::Port => Focus::Bind,
        Focus::Bind => Focus::Root,
    }
}

fn prev_focus(f: Focus) -> Focus {
    match f {
        Focus::Root => Focus::Bind,
        Focus::Port => Focus::Root,
        Focus::Bind => Focus::Port,
    }
}

fn edit_target(state: &mut State, field: Focus) -> &mut String {
    match field {
        Focus::Root => &mut state.root,
        Focus::Port => &mut state.port,
        Focus::Bind => &mut state.bind,
    }
}

fn on_key(state: &mut State, key: KeyEvent) -> KeyAction {
    if key.kind != KeyEventKind::Press {
        return KeyAction::Ignored;
    }
    // Ctrl+C / Ctrl+Q always quit — even mid-edit.
    if key.modifiers.contains(KeyModifiers::CONTROL)
        && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('q'))
    {
        return KeyAction::Quit;
    }

    match state.focus {
        // ── Command mode: letters are commands, nothing types. ──────────
        None => match key.code {
            KeyCode::Char('q') => KeyAction::Quit,
            KeyCode::Char('s') => {
                // Start when idle/failed; a running server is left alone.
                if state.status != Status::Running && state.status != Status::Starting {
                    state.start();
                }
                KeyAction::Consumed
            }
            KeyCode::Char('x') => {
                if state.status == Status::Running {
                    state.stop();
                }
                KeyAction::Consumed
            }
            KeyCode::Tab | KeyCode::BackTab => {
                state.focus = Some(Focus::Root);
                KeyAction::Consumed
            }
            _ => KeyAction::Ignored,
        },
        // ── Edit mode: every printable key types; q/s/x are NOT commands. ──
        Some(field) => match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                state.focus = None;
                KeyAction::Consumed
            }
            KeyCode::Tab | KeyCode::Down => {
                state.focus = Some(next_focus(field));
                KeyAction::Consumed
            }
            KeyCode::BackTab | KeyCode::Up => {
                state.focus = Some(prev_focus(field));
                KeyAction::Consumed
            }
            KeyCode::Backspace => {
                edit_target(state, field).pop();
                KeyAction::Consumed
            }
            KeyCode::Char(c) => {
                // Restrict port to digits, allow everything else for root/bind.
                if field == Focus::Port && !c.is_ascii_digit() {
                    return KeyAction::Ignored;
                }
                edit_target(state, field).push(c);
                KeyAction::Consumed
            }
            _ => KeyAction::Ignored,
        },
    }
}

// ─── Live dashboard feed ────────────────────────────────────────────────────
//
// The in-process server doesn't hand us its stats or log directly, so we poll
// its own HTTP endpoints — `/api/stats` and `/api/log` — once a second, the
// same way the GUI does. Raw TcpStream keeps the TUI dependency-free; the
// requests are localhost, unauthenticated, and each one times out after 2 s.
// The task dies with the runtime when the server is stopped.

async fn poll_loop(base_url: String, live: Arc<Mutex<LiveData>>) {
    let mut last_log: Vec<LogLine> = Vec::new();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tick.tick().await;

        let url = format!("{base_url}/api/stats");
        let stats = tokio::task::spawn_blocking(move || http_get_json::<StatsSnapshot>(&url)).await;
        if let Ok(Ok(s)) = stats {
            live.lock().stats = s;
        }

        let url = format!("{base_url}/api/log?tail=500");
        let log = tokio::task::spawn_blocking(move || http_get_json::<LogResponse>(&url)).await;
        let Ok(Ok(resp)) = log else { continue };
        let fresh = new_log_lines(&last_log, &resp.lines);
        if !fresh.is_empty() {
            let mut g = live.lock();
            for l in &fresh {
                g.push_log(log_line_string(l));
            }
        }
        last_log = resp.lines;
    }
}

/// Lines in `new` that the UI hasn't appended yet. The core buffer is an
/// append-only ring, so the newest line we already have must appear in the
/// new tail — everything after it is fresh. Matching on `(ts_ms, msg)` keeps
/// same-millisecond events from being dropped or duplicated.
fn new_log_lines(prev: &[LogLine], new: &[LogLine]) -> Vec<LogLine> {
    // First poll of a fresh server: its whole buffer is new.
    let Some(last) = prev.last() else {
        return new.to_vec();
    };
    match new
        .iter()
        .rposition(|l| l.ts_ms == last.ts_ms && l.msg == last.msg)
    {
        Some(idx) => new[idx + 1..].to_vec(),
        // The ring rotated past our last line — hand over the whole tail
        // rather than silently dropping events.
        None => new.to_vec(),
    }
}

fn log_line_string(l: &LogLine) -> String {
    let t = l.ts_ms / 1000 % 86400;
    let h = t / 3600;
    let m = (t % 3600) / 60;
    let s = t % 60;
    format!("[{h:02}:{m:02}:{s:02}] {:<5} {}", l.level, l.msg)
}

/// Minimal JSON-GET over a raw TCP stream. The server is local and has no
/// auth; pulling in an HTTP client for one tiny poll isn't worth it. The
/// streaming `Deserializer` stops at the first JSON value, so trailing
/// chunked-encoding bytes (`\r\n0\r\n\r\n`) are tolerated.
fn http_get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))
        .unwrap_or(url);
    let host_port = without_scheme
        .split('/')
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "bad url".to_string())?;
    let path = without_scheme
        .find('/')
        .map(|i| &without_scheme[i..])
        .unwrap_or("/");

    let mut stream = TcpStream::connect(host_port).map_err(|e| format!("connect: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(2))).ok();
    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\nUser-Agent: qshare\r\n\r\n"
    );
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    stream.flush().ok();
    let mut buf = Vec::with_capacity(2048);
    stream
        .read_to_end(&mut buf)
        .map_err(|e| format!("read: {e}"))?;
    let sep = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "no body separator".to_string())?;
    let body = &buf[sep + 4..];
    let body_start = body
        .iter()
        .position(|&b| b == b'{' || b == b'[')
        .unwrap_or(0);
    let mut de = serde_json::Deserializer::from_slice(&body[body_start..]).into_iter::<T>();
    match de.next() {
        Some(Ok(v)) => Ok(v),
        Some(Err(e)) => Err(format!("parse: {e}")),
        None => Err("parse: empty body".into()),
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_target(false)
        .init();

    let args = Args::parse();
    let mut state = State::new(args);

    let mut terminal = ratatui::init();
    let res = run_app(&mut terminal, &mut state);
    ratatui::restore();
    if state.status == Status::Running {
        state.stop();
    }
    res
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut State,
) -> anyhow::Result<()> {
    // Redraw at ~10 FPS for the "uptime" counter to tick.
    let mut last_tick = Instant::now();
    loop {
        // Tick the flame animation every ~150 ms so the four-frame cycle
        // completes in ~600 ms — fast enough to read as a flicker, slow
        // enough that the eye actually sees each pose. Coupled to the
        // redraw cadence rather than wall-clock so the TUI stays cheap
        // when nothing else needs repainting.
        if last_tick.elapsed() >= Duration::from_millis(150) {
            state.flame_frame = state.flame_frame.wrapping_add(1);
        }
        terminal.draw(|f| view(state, f))?;
        let timeout = Duration::from_millis(150).saturating_sub(last_tick.elapsed());
        let deadline = Instant::now() + timeout.max(Duration::from_millis(50));
        let mut exit = false;
        // Drain all events that are already buffered, then wait until the
        // deadline for one more — this gives us responsiveness without
        // a busy loop.
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }
            if crossterm::event::poll(remaining.min(Duration::from_millis(50)))? {
                if let Event::Key(k) = crossterm::event::read()? {
                    if on_key(state, k) == KeyAction::Quit {
                        exit = true;
                        break;
                    }
                }
            } else {
                break;
            }
        }
        if exit {
            return Ok(());
        }
        last_tick = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn state() -> State {
        let args = Args {
            root: Some(PathBuf::from("/tmp")),
            root_flag: None,
            port: 8888,
            bind: "0.0.0.0".into(),
        };
        State::new(args)
    }

    #[test]
    fn command_q_quits() {
        let mut st = state();
        assert_eq!(on_key(&mut st, key(KeyCode::Char('q'))), KeyAction::Quit);
    }

    #[test]
    fn editing_types_q_instead_of_quitting() {
        let mut st = state();
        st.focus = Some(Focus::Root);
        let before = st.root.clone();
        assert_eq!(
            on_key(&mut st, key(KeyCode::Char('q'))),
            KeyAction::Consumed
        );
        assert_eq!(st.root, format!("{before}q"));
    }

    #[test]
    fn editing_types_s_instead_of_starting() {
        let mut st = state();
        st.focus = Some(Focus::Root);
        let before = st.root.clone();
        assert_eq!(
            on_key(&mut st, key(KeyCode::Char('s'))),
            KeyAction::Consumed
        );
        assert_eq!(st.root, format!("{before}s"));
        assert_eq!(st.status, Status::Idle);
    }

    #[test]
    fn ctrl_c_always_quits_even_mid_edit() {
        let mut st = state();
        st.focus = Some(Focus::Port);
        assert_eq!(
            on_key(
                &mut st,
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
            ),
            KeyAction::Quit
        );
    }

    #[test]
    fn tab_enters_edit_then_cycles_fields() {
        let mut st = state();
        // Command → edit root
        assert_eq!(on_key(&mut st, key(KeyCode::Tab)), KeyAction::Consumed);
        assert_eq!(st.focus, Some(Focus::Root));
        // Cycle port → bind
        assert_eq!(on_key(&mut st, key(KeyCode::Tab)), KeyAction::Consumed);
        assert_eq!(st.focus, Some(Focus::Port));
        assert_eq!(on_key(&mut st, key(KeyCode::Tab)), KeyAction::Consumed);
        assert_eq!(st.focus, Some(Focus::Bind));
    }

    #[test]
    fn esc_and_enter_leave_edit_mode() {
        let mut st = state();
        st.focus = Some(Focus::Port);
        assert_eq!(on_key(&mut st, key(KeyCode::Esc)), KeyAction::Consumed);
        assert_eq!(st.focus, None);
        st.focus = Some(Focus::Bind);
        assert_eq!(on_key(&mut st, key(KeyCode::Enter)), KeyAction::Consumed);
        assert_eq!(st.focus, None);
    }

    #[test]
    fn port_editing_rejects_letters() {
        let mut st = state();
        st.focus = Some(Focus::Port);
        let before = st.port.clone();
        assert_eq!(on_key(&mut st, key(KeyCode::Char('a'))), KeyAction::Ignored);
        assert_eq!(st.port, before);
    }

    #[test]
    fn stop_only_works_in_command_mode_when_running() {
        let mut st = state();
        st.status = Status::Running;
        assert_eq!(
            on_key(&mut st, key(KeyCode::Char('x'))),
            KeyAction::Consumed
        );
        assert_eq!(st.status, Status::Idle);
    }

    #[test]
    fn up_down_cycles_fields() {
        let mut st = state();
        st.focus = Some(Focus::Port);
        // Down → bind
        assert_eq!(on_key(&mut st, key(KeyCode::Down)), KeyAction::Consumed);
        assert_eq!(st.focus, Some(Focus::Bind));
        // Up → back to port
        assert_eq!(on_key(&mut st, key(KeyCode::Up)), KeyAction::Consumed);
        assert_eq!(st.focus, Some(Focus::Port));
        // Up wraps to root
        assert_eq!(on_key(&mut st, key(KeyCode::Up)), KeyAction::Consumed);
        assert_eq!(st.focus, Some(Focus::Root));
    }

    #[test]
    fn new_log_lines_dedups_across_polls() {
        let one = LogLine {
            ts_ms: 1000,
            level: "info".into(),
            msg: "one".into(),
        };
        let two = LogLine {
            ts_ms: 1000,
            level: "info".into(),
            msg: "two".into(),
        };
        let prev = vec![one.clone(), two.clone()];
        let next = vec![
            one,
            two,
            LogLine {
                ts_ms: 3000,
                level: "warn".into(),
                msg: "three".into(),
            },
        ];
        let fresh = new_log_lines(&prev, &next);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].msg, "three");
    }
}
