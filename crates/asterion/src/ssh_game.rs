//! Glue between `frittura-ssh-core`'s `SshGame` trait and asterion's central
//! game task. Each new SSH session gets a fresh `PlayerId` (UUID) - asterion
//! is stateless per session and doesn't validate credentials.

use crate::server_loop;
use crate::tui::Tui;
use crate::PlayerId;
use frittura_ssh_core::{spawn_event_converter, Credential, SshGame, SshSession, TerminalEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

/// App-level idle kick, distinct from `SshGame::SERVER_INACTIVITY` (russh
/// protocol-level connection timeout). Player gets a 10s warning before kick.
const APP_IDLE_KICK: Duration = Duration::from_secs(60);
const APP_IDLE_WARNING: Duration = Duration::from_secs(10);

pub struct AsterionGame {
    client_sender: mpsc::Sender<Tui>,
    terminal_event_sender: mpsc::Sender<(PlayerId, TerminalEvent)>,
}

impl AsterionGame {
    pub fn new() -> Arc<Self> {
        let (client_sender, client_receiver) = mpsc::channel(16);
        let (terminal_event_sender, terminal_event_receiver) = mpsc::channel(64);
        server_loop::spawn(client_receiver, terminal_event_receiver);
        Arc::new(Self {
            client_sender,
            terminal_event_sender,
        })
    }
}

impl SshGame for AsterionGame {
    type Auth = PlayerId;
    const SCREEN_SIZE: (u16, u16) = (160, 30);
    const TITLE: &'static str = "Asterion";
    const SERVER_INACTIVITY: Duration = Duration::from_secs(120);

    async fn authenticate(
        &self,
        _username: &str,
        _credential: Credential,
    ) -> anyhow::Result<PlayerId> {
        // asterion is stateless: every connection is a new player.
        Ok(Uuid::new_v4())
    }

    async fn on_session(self: Arc<Self>, session: SshSession<PlayerId>) {
        let SshSession {
            username,
            auth: player_id,
            writer,
            data_rx,
            resize_rx,
            ..
        } = session;

        let tui = match Tui::new(player_id, username, writer) {
            Ok(t) => t,
            Err(e) => {
                log::error!("Tui init failed for {player_id}: {e}");
                return;
            }
        };

        if self.client_sender.send(tui).await.is_err() {
            log::warn!("Game task gone; dropping session for {player_id}");
            return;
        }

        // Parse inbound bytes + window-changes into a single TerminalEvent
        // stream via the shared core helper, tagged with `player_id` for the
        // central task.
        let mut events =
            spawn_event_converter(data_rx, resize_rx, Some(APP_IDLE_KICK), Some(APP_IDLE_WARNING));
        let tev_tx = self.terminal_event_sender.clone();
        while let Some(ev) = events.recv().await {
            if tev_tx.send((player_id, ev)).await.is_err() {
                break;
            }
        }
    }
}
