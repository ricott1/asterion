use super::client::{AppClient, PlayerId};
use crate::game::{Game, GameCommand};
use crate::ssh::TerminalEvent;
use crate::tui::Tui;
use crate::AppResult;
use crossterm::event::KeyCode;
use itertools::Either;
use rand::Rng;
use russh::keys::ssh_key::private::{Ed25519Keypair, Ed25519PrivateKey, KeypairData};
use russh::server::{self};
use russh::server::{Config, Server};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::pin::pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::Sender;
use tokio::sync::mpsc::{self, Receiver};
use tokio::task;
use tokio::{select, time};
use tokio_util::sync::CancellationToken;

fn save_keys(signing_key: &russh::keys::PrivateKey) -> AppResult<()> {
    let file = File::create::<&str>("./keys".into())?;
    assert!(file.metadata()?.is_file());
    let mut buffer = std::io::BufWriter::new(file);
    buffer.write(&signing_key.to_bytes()?)?;
    println!("Created new keypair for SSH server.");
    Ok(())
}

fn load_keys() -> AppResult<russh::keys::PrivateKey> {
    let bytes = std::fs::read("./keys")?;
    let private_key = russh::keys::PrivateKey::from_bytes(&bytes)?;
    println!("Loaded keypair for SSH server.");
    Ok(private_key)
}

pub struct AppServer {
    port: u16,
    shutdown: CancellationToken,
    client_sender: Option<Sender<Tui>>,
    terminal_event_sender: Option<Sender<(PlayerId, TerminalEvent)>>,
}

impl AppServer {
    const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60);
    pub fn new(port: u16) -> Self {
        Self {
            port,
            shutdown: CancellationToken::new(),
            client_sender: None,
            terminal_event_sender: None,
        }
    }

    pub async fn run(&mut self) -> AppResult<()> {
        println!(
            "Starting SSH server on port {}. Press Ctrl-C to exit.",
            self.port
        );

        let private_key = load_keys().unwrap_or_else(|_| {
            let rng = &mut rand::rng();
            let seed: [u8; Ed25519PrivateKey::BYTE_SIZE] = rng.random();
            let key_data = KeypairData::from(Ed25519Keypair::from_seed(&seed));
            let key = russh::keys::PrivateKey::new(key_data, "Rebels ssh server key")
                .expect("Failed to generate SSH keys.");

            save_keys(&key).expect("Failed to save SSH keys.");
            key
        });

        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(120)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![private_key],
            ..Default::default()
        };

        let shutdown = self.shutdown.clone();

        let (client_sender, client_receiver) = mpsc::channel(1);
        self.client_sender = Some(client_sender);

        let (terminal_event_sender, terminal_event_receiver) = mpsc::channel(1);
        self.terminal_event_sender = Some(terminal_event_sender);

        Self::spawn_game(
            client_receiver,
            terminal_event_receiver,
            self.shutdown.clone(),
        );

        let server = self.run_on_address(Arc::new(config), ("0.0.0.0", self.port));

        let shutdown_cancelled = shutdown.cancelled();

        let result = {
            let mut server = pin!(server);
            let mut shutdown_cancelled = pin!(shutdown_cancelled);
            select! {
                result = &mut server => Either::Left(result),
                _ = &mut shutdown_cancelled => Either::Right(()),
            }
        };

        match result {
            Either::Left(result) => Ok(result?),
            Either::Right(_) => {
                println!("Shutting down");
                time::sleep(Duration::from_secs(1)).await;

                Ok(())
            }
        }
    }

    fn spawn_game(
        mut client_receiver: Receiver<Tui>,
        mut terminal_event_receiver: Receiver<(PlayerId, TerminalEvent)>,
        server_shutdown: CancellationToken,
    ) {
        task::spawn(async move {
            let mut game = match Game::new() {
                Ok(g) => g,
                Err(err) => {
                    log::error!("Unable to spawn game: {err}");
                    return;
                }
            };
            let mut update_ticker = tokio::time::interval(Game::update_time_step());
            let mut draw_ticker = tokio::time::interval(Game::draw_time_step());

            let mut tuis: HashMap<PlayerId, Tui> = HashMap::new();
            let mut last_moves: HashMap<PlayerId, Instant> = HashMap::new();

            loop {
                select! {
                    Some(tui) = client_receiver.recv() => {
                        game.add_player(tui.id,tui.username());
                        last_moves.insert(tui.id, Instant::now());
                        tuis.insert(tui.id, tui);
                    }

                    _ = update_ticker.tick() => {
                        game.update();
                    }

                    _ = draw_ticker.tick() => {
                        let mut to_remove = vec![];
                        for (&player_id, tui) in tuis.iter_mut() {
                            tui.draw(&game).expect("Can't draw tui");
                            if let Err(e) = tui.push_data().await {
                                println!("Error pushing to tui: {}", e);
                                let _ = tui.exit().await;
                                to_remove.push(player_id);
                            } else if let Some(last_move) = last_moves.get(&player_id) {
                                if last_move.elapsed() > Self::INACTIVITY_TIMEOUT {
                                    let _ = tui.exit().await;
                                    to_remove.push(player_id);
                                }
                            }
                        }
                        for player_id in to_remove {
                            game.remove_player(&player_id);
                            tuis.remove(&player_id);
                        }
                    }

                    Some((player_id, event)) = terminal_event_receiver.recv() => {
                        last_moves.insert(player_id, Instant::now());
                        match event {
                            TerminalEvent::Key{key_event} => {
                                match key_event.code {
                                    KeyCode::Char('q') | KeyCode::Esc => {
                                        game.remove_player(&player_id);

                                        if let Some(tui) = tuis.get_mut(&player_id) {
                                            let _ = tui.exit().await;
                                        }
                                        tuis.remove(&player_id);
                                    }

                                    code => {
                                        if let Some(command) = GameCommand::from_key_code(code) {
                                            game.handle_command(&command, player_id);
                                        }
                                    }
                                }
                            }

                            TerminalEvent::Resize{width, height} => {
                                if let Some(tui) = tuis.get_mut(&player_id).as_mut() {
                                    let _ = tui.resize(width, height);
                                }
                            },
                            _ => {}
                        }
                    }

                    _ = server_shutdown.cancelled() => {
                        log::info!("Shutting down game from server.");
                        break
                    }

                }
            }

            // Game has ended.
            for tui in tuis.values_mut() {
                let _ = tui.exit().await;
            }
        });
    }
}

impl server::Server for AppServer {
    type Handler = AppClient;
    fn new_client(&mut self, _peer_addr: Option<std::net::SocketAddr>) -> AppClient {
        let client_sender = self
            .client_sender
            .as_ref()
            .expect("Tui sender should have been initialized")
            .clone();

        let terminal_event_sender = self
            .terminal_event_sender
            .as_ref()
            .expect("Tui sender should have been initialized")
            .clone();
        let client = AppClient::new(self.shutdown.clone(), client_sender, terminal_event_sender);

        client
    }
}
