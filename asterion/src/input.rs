use asterion_core::{Direction, GameCommand};
use ratatui::crossterm::event::KeyCode;

pub fn key_to_command(key_code: KeyCode) -> Option<GameCommand> {
    match key_code {
        KeyCode::Char(c) => match c {
            'a' => Some(GameCommand::TurnCounterClockwise),
            'd' => Some(GameCommand::TurnClockwise),
            'w' => Some(GameCommand::CycleUiOptions),
            'h' => Some(GameCommand::Move {
                direction: Direction::West,
            }),
            'j' => Some(GameCommand::Move {
                direction: Direction::South,
            }),
            'k' => Some(GameCommand::Move {
                direction: Direction::North,
            }),
            'l' => Some(GameCommand::Move {
                direction: Direction::East,
            }),
            _ => None,
        },
        KeyCode::Up => Some(GameCommand::Move {
            direction: Direction::North,
        }),
        KeyCode::Down => Some(GameCommand::Move {
            direction: Direction::South,
        }),
        KeyCode::Left => Some(GameCommand::Move {
            direction: Direction::West,
        }),
        KeyCode::Right => Some(GameCommand::Move {
            direction: Direction::East,
        }),
        _ => None,
    }
}
