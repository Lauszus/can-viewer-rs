use socketcan::tokio::CanSocket;
use std::collections::HashMap;
use std::env;
use std::time::Instant;

use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame as AppFrame,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};
use socketcan::{CanFrame, EmbeddedFrame, Frame};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new()?.run(terminal).await;
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
    frame_stats: HashMap<u32, FrameStats>,
    start_time: Instant,
}

impl App {
    #[must_use]
    pub fn new() -> Result<Self> {
        let iface = env::args().nth(1).unwrap_or_else(|| "vcan0".into());
        let can_socket = CanSocket::open(&iface)?;

        Ok(Self {
            running: false,
            paused: false,
            event_stream: EventStream::new(),
            can_socket,
            frame_stats: HashMap::new(),
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

                                let dt = if let Some(stats) = self.frame_stats.get(&frame_id) {
                                    current_time - stats.last_time
                                } else {
                                    0.0
                                };

                                let stats = self.frame_stats.entry(frame_id).or_insert(FrameStats {
                                    count: 0,
                                    last_time: 0.0,
                                    last_dt: 0.0,
                                    last_frame: frame,
                                });

                                stats.count += 1;
                                stats.last_dt = dt;
                                stats.last_time = current_time;
                                stats.last_frame = frame;
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
        let title = Line::from("CAN Frame Monitor").bold().blue().centered();

        let mut lines = vec!["Count   Time        dt         ID          DLC  Data".to_string()];

        // Sort by frame ID for consistent display
        let mut sorted_frames: Vec<_> = self.frame_stats.iter().collect();
        sorted_frames.sort_by_key(|(id, _)| *id);

        for (id, stats) in sorted_frames {
            let data_hex = stats
                .last_frame
                .data()
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");

            let line = format!(
                "{:<7} {:<11.6} {:<10.6} {:<11} {:<3} {}",
                stats.count,
                stats.last_time,
                stats.last_dt,
                if stats.last_frame.is_extended() {
                    format!("0x{:08X}", id)
                } else {
                    format!("0x{:03X}", id)
                },
                stats.last_frame.dlc(),
                data_hex
            );
            lines.push(line);
        }

        let text = if lines.len() == 1 {
            "Waiting for CAN frames...".to_string()
        } else {
            lines.join("\n")
        };

        frame.render_widget(
            Paragraph::new(text)
                .block(Block::bordered().title(title))
                .scroll((
                    lines.len().saturating_sub(frame.area().height as usize - 2) as u16,
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
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
