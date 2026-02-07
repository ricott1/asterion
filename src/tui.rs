use crate::constants::UI_SCREEN_SIZE;
use crate::game::Game;
use crate::ssh::SSHWriterProxy;
use crate::ui;
use crate::AppResult;
use crate::PlayerId;
use crossterm::cursor::{Hide, Show};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::Clear;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::Rect;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use ratatui::TerminalOptions;
use ratatui::Viewport;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

#[derive(Debug)]
pub struct Tui {
    pub id: PlayerId,
    username: String,
    start_instant: Instant,
    terminal: Terminal<CrosstermBackend<SSHWriterProxy>>,
    client_shutdown: CancellationToken,
}

impl Tui {
    fn init(&mut self) -> AppResult<()> {
        crossterm::execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture,
            Clear(crossterm::terminal::ClearType::All),
            Hide
        )?;

        Ok(())
    }

    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn new(
        id: PlayerId,
        username: String,
        writer: SSHWriterProxy,
        client_shutdown: CancellationToken,
    ) -> AppResult<Self> {
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
            client_shutdown,
        };

        tui.init()?;

        Ok(tui)
    }

    pub fn draw(&mut self, game: &Game) -> AppResult<()> {
        self.terminal.draw(|frame| {
            ui::ui::render(frame, game, self.id, self.start_instant)
                .expect("Error while rendering game.")
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

    pub async fn exit(&mut self) -> AppResult<()> {
        self.client_shutdown.cancel();

        crossterm::execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            Clear(crossterm::terminal::ClearType::All),
            Show
        )?;

        self.terminal.backend_mut().writer_mut().send().await?;
        self.terminal.backend_mut().writer_mut().exit().await;
        Ok(())
    }
}
