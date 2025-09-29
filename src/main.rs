mod style;

use clap::Parser;
use color_eyre::Result;
use colored::Colorize;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use indexmap::IndexMap;
use ratatui::{
    DefaultTerminal, Frame as AppFrame,
    layout::{Alignment, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
};
use socketcan::tokio::CanSocket;
use socketcan::{CanFrame, EmbeddedFrame, Frame};
use std::time::{Duration, Instant};

const KEYBOARD_SHORTCUTS: &str = r"
  ┌──────────┬──────────────────────────┐
  │   Key    │         Description      │
  ├──────────┼──────────────────────────┤
  │ ESC/q    │ Quit                     │
  │ SPACE    │ Pause/Resume             │
  │ s        │ Sort frames              │
  │ c        │ Clear frames             │
  │ ↑↓       │ Scroll                   │
  │ h        │ Toggle help (this popup) │
  └──────────┴──────────────────────────┘
";

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "A terminal-based CAN bus viewer for Linux SocketCAN written in Rust",
    long_about = None,
    after_help = format!("Shortcuts:{KEYBOARD_SHORTCUTS}"),
)]
#[clap(styles = style::CARGO_STYLING)]
struct Args {
    /// CAN interface to use
    #[arg(short, long)]
    channel: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();
    let can_socket = open_can_socket(&args.channel);

    let app = App::new(can_socket);
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();

    result
}

fn open_can_socket(channel: &String) -> CanSocket {
    match CanSocket::open(channel) {
        Ok(socket) => socket,
        Err(e) => {
            eprintln!(
                "{} '{}':",
                Colorize::bold("Failed to open CAN interface").red(),
                Colorize::yellow(channel.as_str())
            );
            eprintln!("  - {e}");
            eprintln!();
            eprintln!("{}:", Colorize::bold("Please check that").cyan());
            eprintln!(
                "  - The interface exists (try: {})",
                Colorize::green(format!("ip link show {channel}").as_str())
            );
            eprintln!("  - You have sufficient permissions");
            eprintln!(
                "  - The interface is up: {}",
                Colorize::green(
                    format!(
                        "sudo ip link add dev {channel} type vcan && sudo ip link set up {channel}"
                    )
                    .as_str()
                )
            );
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Clone)]
struct FrameStats {
    count: u32,
    dt: f64,
    last_time: f64,
    frame: CanFrame,
}

#[derive(Debug)]
pub struct App {
    running: bool,
    paused: bool,
    show_shortcuts: bool,
    event_stream: EventStream,
    can_socket: CanSocket,
    frame_stats: IndexMap<(u32, usize), FrameStats>,
    start_time: Instant,
    frame_rate: f64,
    scroll_offset: u16, // Track scroll position
}

impl App {
    #[must_use]
    pub fn new(can_socket: CanSocket) -> Self {
        Self {
            running: false,
            paused: false,
            show_shortcuts: false,
            event_stream: EventStream::new(),
            can_socket,
            frame_stats: IndexMap::new(),
            start_time: Instant::now(),
            frame_rate: 60.0, // 60 FPS
            scroll_offset: 0,
        }
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        let render_interval = Duration::from_secs_f64(1.0 / self.frame_rate);
        let mut last_update: Option<Instant> = None;
        while self.running {
            if last_update.is_none_or(|last| last.elapsed() >= render_interval) {
                terminal.draw(|frame| self.draw(frame))?;
                last_update = Some(Instant::now());
            }
            self.handle_events().await?;
        }
        Ok(())
    }

    async fn handle_events(&mut self) -> Result<()> {
        tokio::select! {
            event = self.event_stream.next().fuse() => {
                if let Some(Ok(evt)) = event {
                    match evt {
                        Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                        _ => {}
                    }
                }
            }
            can_result = self.can_socket.next().fuse() => {
                if let Some(res) = can_result {
                    match res {
                        Ok(frame) => {
                            if !self.paused {
                                let current_time = self.start_time.elapsed().as_secs_f64();
                                let frame_id = frame.id_word();
                                let frame_len = frame.len();

                                // Frames are considered the same if they have the same ID and length
                                if let Some(stats) = self.frame_stats.get_mut(&(frame_id, frame_len)) {
                                    stats.count += 1;
                                    stats.dt = current_time - stats.last_time;
                                    stats.last_time = current_time;
                                    stats.frame = frame;
                                } else {
                                   self.frame_stats.insert((frame_id, frame_len), FrameStats {
                                        count: 1,
                                        dt: 0.0,
                                        last_time: current_time,
                                        frame,
                                    });
                                }
                            }
                        }
                        Err(_err) => {
                            // Handle error if needed
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut AppFrame) {
        let header = Line::from("Count   Time           dt          ID          DLC  Data").bold();
        let mut lines = vec![header];

        for stats in self.frame_stats.values() {
            let id = stats.frame.can_id().as_raw();
            let data_hex = stats
                .frame
                .data()
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");

            let line_text = format!(
                "{:<7} {:<14.6} {:<11.6} {:<11} {:<4} {}",
                stats.count,
                stats.last_time,
                stats.dt,
                if stats.frame.is_extended() {
                    format!("0x{id:08X}")
                } else {
                    format!("0x{id:03X}")
                },
                stats.frame.len(),
                data_hex
            );

            // Highlight error frames in red
            if stats.frame.is_error_frame() {
                lines.push(Line::from(line_text.red()));
            } else {
                lines.push(Line::from(line_text));
            }
        }

        // Limit scrolling, so the maximum scrolling position is one below the last line
        let max_scroll =
            u16::try_from(lines.len().saturating_sub(frame.area().height as usize - 1))
                .unwrap_or(0);
        if self.scroll_offset > max_scroll {
            self.scroll_offset = max_scroll;
        }

        frame.render_widget(
            Paragraph::new(lines.clone()).scroll((self.scroll_offset, 0)),
            frame.area(),
        );

        // Draw shortcuts help overlay
        if self.show_shortcuts {
            Self::draw_shortcuts_popup(frame);
        }
    }

    fn draw_shortcuts_popup(frame: &mut AppFrame) {
        // Create the shortcuts content with proper table formatting
        let mut lines = Vec::new();
        for line in KEYBOARD_SHORTCUTS.lines() {
            if line.trim().is_empty() {
                continue; // Skip empty lines
            }
            if line.contains("Description") {
                lines.push(Line::from(line.trim()).bold());
            } else {
                lines.push(Line::from(line.trim()));
            }
        }

        // Calculate exact dimensions needed
        let content_width = u16::try_from(lines[0].width()).unwrap();
        let content_height = u16::try_from(lines.len()).unwrap();
        let border_width = 2; // Left and right borders
        let border_height = 2; // Top and bottom borders

        let popup_width = content_width + border_width;
        let popup_height = content_height + border_height;

        // Create a rectangle for the popup in the top-right corner
        let popup_area = Self::sized_top_right_rect(popup_width, popup_height, frame.area());

        // Clear the area for the popup
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Shortcuts ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Left);

        frame.render_widget(paragraph, popup_area);
    }

    /// Helper function to create a centered rectangle with exact dimensions
    fn sized_top_right_rect(width: u16, height: u16, r: Rect) -> Rect {
        let x = r.width.saturating_sub(width);
        let y = 0;

        Rect {
            x,
            y,
            width: width.min(r.width),
            height: height.min(r.height),
        }
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Up) => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                }
            }
            (_, KeyCode::Down) => {
                self.scroll_offset += 1;
            }
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char(' ')) => {
                self.paused = !self.paused;
            }
            (_, KeyCode::Char('s' | 'S')) => {
                // Note this is sorted by the "id_word" (canid_t), which includes the EFF/RTR/ERR flags
                // This means standard frames (0x000 to 0x7FF) will be sorted before extended frames
                self.frame_stats.sort_by_key(|(id, _), _| *id);
            }
            (_, KeyCode::Char('c' | 'C')) => {
                self.frame_stats.clear();
                self.start_time = Instant::now();
                self.scroll_offset = 0;
            }
            (_, KeyCode::Char('h' | 'H')) => {
                self.show_shortcuts = !self.show_shortcuts;
            }
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
