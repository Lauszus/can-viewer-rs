use color_eyre::Result;
use colored::Colorize;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{DefaultTerminal, Frame as AppFrame, style::Stylize, text::Line, widgets::Paragraph};
use socketcan::tokio::CanSocket;
use socketcan::{CanFrame, EmbeddedFrame, Frame};
use std::env;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let app = App::new()?;
    let terminal = ratatui::init();
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}

#[derive(Debug, Clone)]
struct FrameStats {
    count: u32,
    last_time: f64,
    last_dt: f64,
    last_frame: CanFrame,
}

#[derive(Debug)]
pub struct App {
    running: bool,
    paused: bool,
    event_stream: EventStream,
    can_socket: CanSocket,
    frame_stats: Vec<(u32, FrameStats)>,
    start_time: Instant,
}

impl App {
    pub fn new() -> Result<Self> {
        let iface = env::args().nth(1).unwrap_or_else(|| "vcan0".into());
        let can_socket = match CanSocket::open(&iface) {
            Ok(socket) => socket,
            Err(e) => {
                eprintln!(
                    "{} '{}':",
                    Colorize::bold("Failed to open CAN interface").red(),
                    iface.clone().yellow()
                );
                eprintln!("  {}", e.to_string().red());
                eprintln!();
                eprintln!("{}:", Colorize::bold("Please check that").cyan());
                eprintln!(
                    "  - The interface exists (try: {})",
                    format!("ip link show {iface}").green()
                );
                eprintln!("  - You have sufficient permissions");
                eprintln!(
                    "  - The interface is up: {}",
                    format!(
                        "sudo ip link add dev {iface} type vcan && sudo ip link set up {iface}"
                    )
                    .green()
                );

                std::process::exit(1);
            }
        };

        Ok(Self {
            running: false,
            paused: false,
            event_stream: EventStream::new(),
            can_socket,
            frame_stats: Vec::new(),
            start_time: Instant::now(),
        })
    }

    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.draw(frame))?;
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
                                let frame_id = frame.can_id().as_raw();

                                if let Some((_, stats)) = self.frame_stats.iter_mut().find(|(id, _)| *id == frame_id) {
                                    let dt = current_time - stats.last_time;
                                    stats.count += 1;
                                    stats.last_dt = dt;
                                    stats.last_time = current_time;
                                    stats.last_frame = frame;
                                } else {
                                    self.frame_stats.push((frame_id, FrameStats {
                                        count: 1,
                                        last_time: current_time,
                                        last_dt: 0.0,
                                        last_frame: frame,
                                    }));
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
        let header = Line::from("Count   Time        dt         ID          DLC  Data").bold();
        let mut lines = vec![header];

        for (id, stats) in &self.frame_stats {
            let data_hex = stats
                .last_frame
                .data()
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");

            let line_text = format!(
                "{:<7} {:<11.6} {:<10.6} {:<11} {:<3} {}",
                stats.count,
                stats.last_time,
                stats.last_dt,
                if stats.last_frame.is_extended() {
                    format!("0x{id:08X}")
                } else {
                    format!("0x{id:03X}")
                },
                stats.last_frame.dlc(),
                data_hex
            );
            lines.push(Line::from(line_text));
        }

        let text = if lines.len() == 1 {
            vec![Line::from("Waiting for CAN frames...")]
        } else {
            lines.clone()
        };

        frame.render_widget(
            Paragraph::new(text).scroll((
                u16::try_from(lines.len().saturating_sub(frame.area().height as usize - 2))
                    .unwrap(),
                0,
            )),
            frame.area(),
        );
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char('p' | 'P' | ' ')) => {
                self.paused = !self.paused;
            }
            (_, KeyCode::Char('s' | 'S')) => {
                self.frame_stats.sort_by_key(|(id, _)| *id);
            }
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
