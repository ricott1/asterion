use crate::constants::UI_SCREEN_SIZE;
use crate::ui;
use crate::AppResult;
use crate::PlayerId;
use asterion_core::Game;
use frittura_ssh_core::{idle_warning_text, SshWriterProxy};
use ratatui::crossterm::cursor::Hide;
use ratatui::crossterm::event::EnableMouseCapture;
use ratatui::crossterm::terminal::Clear;
use ratatui::crossterm::terminal::EnterAlternateScreen;
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::style::Style;
use ratatui::widgets::{Block, Clear as ClearWidget, Paragraph};
use ratatui::Terminal;
use ratatui::TerminalOptions;
use ratatui::Viewport;
use std::time::Instant;

#[derive(Debug)]
pub struct Tui {
    pub id: PlayerId,
    username: String,
    start_instant: Instant,
    terminal: Terminal<CrosstermBackend<SshWriterProxy>>,
}

impl Tui {
    fn init(&mut self) -> AppResult<()> {
        ratatui::crossterm::execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture,
            Clear(ratatui::crossterm::terminal::ClearType::All),
            Hide
        )?;
        Ok(())
    }

    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn new(id: PlayerId, username: String, writer: SshWriterProxy) -> AppResult<Self> {
        let backend = CrosstermBackend::new(writer);
        let opts = TerminalOptions {
            viewport: Viewport::Fixed(Rect {
                x: 0,
                y: 0,
                width: UI_SCREEN_SIZE.0,
                height: UI_SCREEN_SIZE.1,
            }),
        };
        let terminal = Terminal::with_options(backend, opts)?;
        let mut tui = Self {
            id,
            username,
            start_instant: Instant::now(),
            terminal,
        };
        tui.init()?;
        Ok(tui)
    }

    pub fn draw(&mut self, game: &Game, idle_warning: Option<u32>) -> AppResult<()> {
        let id = self.id;
        let start = self.start_instant;
        self.terminal.draw(|frame| {
            ui::ui::render(frame, game, id, start).expect("Error while rendering game.");
            if let Some(secs) = idle_warning {
                let area = frame.area();
                let banner_w: u16 = 50;
                let banner_h: u16 = 3;
                let banner = Rect {
                    x: area.x + area.width.saturating_sub(banner_w) / 2,
                    y: area.y + area.height.saturating_sub(banner_h) / 2,
                    width: banner_w.min(area.width),
                    height: banner_h.min(area.height),
                };
                frame.render_widget(ClearWidget, banner);
                frame.render_widget(
                    Paragraph::new(idle_warning_text(secs))
                        .centered()
                        .style(Style::new().red().bold())
                        .block(Block::bordered()),
                    banner,
                );
            }
        })?;
        Ok(())
    }

    pub async fn push_data(&mut self) -> AppResult<()> {
        self.terminal.backend_mut().writer_mut().send().await?;
        Ok(())
    }

    pub fn resize(&mut self, width: u16, height: u16) -> AppResult<()> {
        self.terminal.resize(Rect {
            x: 0,
            y: 0,
            width,
            height,
        })?;
        Ok(())
    }

    /// Restore the terminal and close the SSH channel, awaited end-to-end.
    pub async fn close(mut self) {
        self.terminal.backend_mut().writer_mut().send_and_close().await;
    }
}
