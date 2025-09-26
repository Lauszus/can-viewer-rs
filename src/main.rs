use socketcan::tokio::CanSocket;
use std::env;

use color_eyre::Result;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures::{FutureExt, StreamExt};
use ratatui::{
    DefaultTerminal, Frame,
    style::Stylize,
    text::Line,
    widgets::{Block, Paragraph},
};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::new()?.run(terminal).await;
    ratatui::restore();
    result
}

#[derive(Debug)]
pub struct App {
    running: bool,
    event_stream: EventStream,
    can_socket: CanSocket,
    can_frames: Vec<String>,
}

impl App {
    #[must_use]
    pub fn new() -> Result<Self> {
        let iface = env::args().nth(1).unwrap_or_else(|| "vcan0".into());
        let can_socket = CanSocket::open(&iface)?;

        Ok(Self {
            running: false,
            event_stream: EventStream::new(),
            can_socket,
            can_frames: Vec::new(),
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
                            let frame_str = format!("{frame:?}");
                            self.can_frames.push(frame_str);
                            // Keep only last 100 frames to prevent memory growth
                            if self.can_frames.len() > 100 {
                                self.can_frames.remove(0);
                            }
                        }
                        Err(err) => {
                            self.can_frames.push(format!("Error: {err}"));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let title = Line::from("CAN Frame Monitor").bold().blue().centered();

        let frames_text = if self.can_frames.is_empty() {
            "Waiting for CAN frames...".to_string()
        } else {
            self.can_frames.join("\n")
        };

        frame.render_widget(
            Paragraph::new(frames_text)
                .block(Block::bordered().title(title))
                .scroll((
                    self.can_frames
                        .len()
                        .saturating_sub(frame.area().height as usize - 2)
                        as u16,
                    0,
                )),
            frame.area(),
        );
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            _ => {}
        }
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
