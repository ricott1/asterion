//! Centralized game-task that owns the `Game` (mazes, heroes, minotaurs)
//! and routes per-player input to it.

use crate::input::key_to_command;
use crate::tui::Tui;
use crate::PlayerId;
use asterion_core::Game;
use frittura_ssh_core::TerminalEvent;
use ratatui::crossterm::event::KeyCode;
use std::collections::HashMap;
use tokio::select;
use tokio::sync::mpsc::Receiver;

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
        let mut idle_warnings: HashMap<PlayerId, u32> = HashMap::new();

        loop {
            select! {
                Some(tui) = client_receiver.recv() => {
                    game.add_player(tui.id, tui.username());
                    tuis.insert(tui.id, tui);
                }

                _ = update_ticker.tick() => {
                    game.update();
                }

                _ = draw_ticker.tick() => {
                    let mut to_remove = vec![];
                    for (&player_id, tui) in tuis.iter_mut() {
                        let warning = idle_warnings.get(&player_id).copied();
                        if let Err(e) = tui.draw(&game, warning) {
                            log::warn!("draw error for {player_id}: {e}");
                            to_remove.push(player_id);
                            continue;
                        }
                        if let Err(e) = tui.push_data().await {
                            log::warn!("push error for {player_id}: {e}");
                            to_remove.push(player_id);
                        }
                    }
                    for player_id in to_remove {
                        remove_player(&mut game, &mut tuis, &mut idle_warnings, player_id).await;
                    }
                }

                Some((player_id, event)) = terminal_event_receiver.recv() => {
                    match event {
                        TerminalEvent::Key(key_event) => {
                            idle_warnings.remove(&player_id);
                            if key_event.code == KeyCode::Esc {
                                remove_player(&mut game, &mut tuis, &mut idle_warnings, player_id).await;
                            } else if let Some(command) = key_to_command(key_event.code) {
                                game.handle_command(&command, player_id);
                            }
                        }
                        TerminalEvent::Resize(width, height) => {
                            if let Some(tui) = tuis.get_mut(&player_id) {
                                let _ = tui.resize(width, height);
                            }
                        }
                        TerminalEvent::IdleWarning(secs) => {
                            idle_warnings.insert(player_id, secs);
                        }
                        TerminalEvent::Quit => {
                            remove_player(&mut game, &mut tuis, &mut idle_warnings, player_id).await;
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
    idle_warnings: &mut HashMap<PlayerId, u32>,
    player_id: PlayerId,
) {
    game.remove_player(&player_id);
    idle_warnings.remove(&player_id);
    if let Some(tui) = tuis.remove(&player_id) {
        tui.close().await;
    }
}
