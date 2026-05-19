//! Centralized game-task that owns the `Game` (mazes, heroes, minotaurs)
//! and routes per-player input to it.

use crate::game::{Game, GameCommand};
use crate::tui::Tui;
use crate::PlayerId;
use frittura_ssh_core::TerminalEvent;
use ratatui::crossterm::event::KeyCode;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::select;
use tokio::sync::mpsc::Receiver;

/// App-level idle kick, distinct from `SshGame::SERVER_INACTIVITY` (russh
/// protocol-level connection timeout). Closure is awaited via `tui.close()`.
const APP_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

pub fn spawn(
    mut client_receiver: Receiver<Tui>,
    mut terminal_event_receiver: Receiver<(PlayerId, TerminalEvent)>,
) {
    tokio::spawn(async move {
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
                    game.add_player(tui.id, tui.username());
                    last_moves.insert(tui.id, Instant::now());
                    tuis.insert(tui.id, tui);
                }

                _ = update_ticker.tick() => {
                    game.update();
                }

                _ = draw_ticker.tick() => {
                    let mut to_remove = vec![];
                    for (&player_id, tui) in tuis.iter_mut() {
                        if let Err(e) = tui.draw(&game) {
                            log::warn!("draw error for {player_id}: {e}");
                            to_remove.push(player_id);
                            continue;
                        }
                        if let Err(e) = tui.push_data().await {
                            log::warn!("push error for {player_id}: {e}");
                            to_remove.push(player_id);
                        } else if let Some(last_move) = last_moves.get(&player_id) {
                            if last_move.elapsed() > APP_IDLE_TIMEOUT {
                                log::info!("idle-kicking {player_id}");
                                to_remove.push(player_id);
                            }
                        }
                    }
                    for player_id in to_remove {
                        remove_player(&mut game, &mut tuis, &mut last_moves, player_id).await;
                    }
                }

                Some((player_id, event)) = terminal_event_receiver.recv() => {
                    last_moves.insert(player_id, Instant::now());
                    match event {
                        TerminalEvent::Key(key_event) => {
                            if key_event.code == KeyCode::Esc {
                                remove_player(&mut game, &mut tuis, &mut last_moves, player_id).await;
                            } else if let Some(command) = GameCommand::from_key_code(key_event.code) {
                                game.handle_command(&command, player_id);
                            }
                        }
                        TerminalEvent::Resize(width, height) => {
                            if let Some(tui) = tuis.get_mut(&player_id) {
                                let _ = tui.resize(width, height);
                            }
                        }
                        TerminalEvent::Quit => {
                            remove_player(&mut game, &mut tuis, &mut last_moves, player_id).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}

async fn remove_player(
    game: &mut Game,
    tuis: &mut HashMap<PlayerId, Tui>,
    last_moves: &mut HashMap<PlayerId, Instant>,
    player_id: PlayerId,
) {
    game.remove_player(&player_id);
    last_moves.remove(&player_id);
    if let Some(tui) = tuis.remove(&player_id) {
        tui.close().await;
    }
}
