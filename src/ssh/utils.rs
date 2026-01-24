use super::SSHEventHandler;
use crate::AppResult;
use anyhow::anyhow;
use crossterm::event::KeyModifiers;

fn convert_data_to_key_event(data: &[u8]) -> Option<crossterm::event::KeyEvent> {
    let key = match data {
        b"\x1b\x5b\x41" => crossterm::event::KeyCode::Up,
        b"\x1b\x5b\x42" => crossterm::event::KeyCode::Down,
        b"\x1b\x5b\x43" => crossterm::event::KeyCode::Right,
        b"\x1b\x5b\x44" => crossterm::event::KeyCode::Left,
        b"\x03" | b"\x1b" => crossterm::event::KeyCode::Esc, // Ctrl-C is also sent as Esc
        b"\x0d" => crossterm::event::KeyCode::Enter,
        b"\x7f" => crossterm::event::KeyCode::Backspace,
        b"\x1b[3~" => crossterm::event::KeyCode::Delete,
        b"\x09" => crossterm::event::KeyCode::Tab,
        x if x.len() == 1 => crossterm::event::KeyCode::Char(data[0] as char),
        _ => {
            return None;
        }
    };
    let event = crossterm::event::KeyEvent::new(key, crossterm::event::KeyModifiers::empty());

    Some(event)
}

fn decode_sgr_mouse_input(ansi_code: Vec<u8>) -> AppResult<(u8, u16, u16)> {
    // Convert u8 vector to a String
    let ansi_str =
        String::from_utf8(ansi_code.clone()).map_err(|_| anyhow!("Invalid UTF-8 sequence"))?;

    // Check the prefix
    if !ansi_str.starts_with("\x1b[<") {
        return Err(anyhow!("Invalid SGR ANSI mouse code"));
    }

    let cb_mod = if ansi_str.ends_with('M') {
        0
    } else if ansi_str.ends_with('m') {
        3
    } else {
        return Err(anyhow!("Invalid SGR ANSI mouse code"));
    };

    // Remove the prefix '\x1b[<' and trailing 'M'
    let code_body = &ansi_str[3..ansi_str.len() - 1];

    // Split the components
    let components: Vec<&str> = code_body.split(';').collect();

    if components.len() != 3 {
        return Err(anyhow!("Invalid SGR ANSI mouse code format"));
    }

    // Parse the components
    let cb = cb_mod
        + components[0]
            .parse::<u8>()
            .map_err(|_| anyhow!("Failed to parse Cb"))?;
    let cx = components[1]
        .parse::<u16>()
        .map_err(|_| anyhow!("Failed to parse Cx"))?
        - 1;
    let cy = components[2]
        .parse::<u16>()
        .map_err(|_| anyhow!("Failed to parse Cy"))?
        - 1;

    Ok((cb, cx, cy))
}

fn convert_data_to_mouse_event(data: &[u8]) -> Option<crossterm::event::MouseEvent> {
    let (cb, column, row) = decode_sgr_mouse_input(data.to_vec()).ok()?;
    let kind = match cb {
        0 => crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
        1 => crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Middle),
        2 => crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right),
        3 => crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left),
        32 => crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Left),
        33 => crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Middle),
        34 => crossterm::event::MouseEventKind::Drag(crossterm::event::MouseButton::Right),
        35 => crossterm::event::MouseEventKind::Moved,
        64 => crossterm::event::MouseEventKind::ScrollUp,
        65 => crossterm::event::MouseEventKind::ScrollDown,
        96..=255 => {
            return None;
        }
        _ => return None,
    };

    let event = crossterm::event::MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    };

    Some(event)
}

pub fn convert_data_to_crossterm_event(data: &[u8]) -> Option<crossterm::event::Event> {
    if let Some(size) = data.strip_prefix(&[SSHEventHandler::CMD_RESIZE]) {
        let cols = size.first().copied().unwrap_or(0) as u16;
        let rows = size.last().copied().unwrap_or(0) as u16;
        return Some(crossterm::event::Event::Resize(cols, rows));
    }

    if data.starts_with(&[27, 91, 60]) {
        if let Some(event) = convert_data_to_mouse_event(data) {
            return Some(crossterm::event::Event::Mouse(event));
        }
    } else if let Some(event) = convert_data_to_key_event(data) {
        return Some(crossterm::event::Event::Key(event));
    }

    None
}
