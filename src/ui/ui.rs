use super::utils::{img_to_lines, RataColor};
use crate::{
    constants::UI_SCREEN_SIZE,
    game::{Entity, Game, GameColors, Hero, Maze, MAX_MAZE_ID, POWER_UPS_PER_ROOM},
    AppResult, PlayerId,
};
use anyhow::anyhow;
use itertools::Itertools;
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Style, Styled},
    symbols::border,
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Wrap},
    Frame,
};
use std::time::{Duration, Instant};

const MINORADAR: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
const NAME_LENGTH: usize = 13;

const TITLE: [&str; 29] = [
    "     ██▓    ▄▄▄          ▄████▄   ▄▄▄        ██████  ▄▄▄            ",
    "     ▓██▒   ▒████▄       ▒██▀  ▀  ▒████▄    ▒██    ▒ ▒████▄         ",
    "     ▒██░   ▒██  ▀█▄     ▒▓█      ▒██  ▀█▄  ░ ▓██▄   ▒██  ▀█▄       ",
    "     ▒██░   ░██▄▄▄▄██    ▒▓▓▄  ▄ ▒░██▄▄▄▄██   ░   ██▒░██▄▄▄▄██      ",
    "     ░██████▒▓█   ▓██▒   ▒ ▓███▀ ░ ▓█   ▓██▒▒██████▒▒ ▓█   ▓██▒     ",
    "     ░ ▒░▓  ░▒▒   ▓▒█░   ░ ░▒ ▒  ░ ▒▒   ▓▒█░▒ ▒▓▒ ▒ ░ ▒▒   ▓▒█░     ",
    "     ░ ░ ▒  ░ ▒   ▒▒ ░     ░  ▒     ▒   ▒▒ ░░ ░▒  ░ ░  ▒   ▒▒ ░     ",
    "       ░ ░    ░   ▒      ░          ░   ▒   ░  ░  ░    ░   ▒        ",
    "         ░  ░     ░  ░   ░ ░            ░  ░      ░        ░  ░     ",
    "                 ░                                                  ",
    "                         ▓█████▄ ▓████▒                             ",
    "                         ▒██▀ ██▌▓█   ▀                             ",
    "                         ░██   █▌▒███                               ",
    "                         ░▓█▄   ▌▒▓█  ▄                             ",
    "                         ░▒████▓ ░▒████▒                            ",
    "                          ▒▒▓  ▒ ░░ ▒░ ░                            ",
    "                          ░ ▒  ▒  ░ ░  ░                            ",
    "                          ░ ░  ░    ░                               ",
    "                            ░       ░  ░             ▄█▓            ",
    "                          ░                         ▀▀▒░            ",
    "▄▄▄        ██████ ▄▄▄█████▓▓█████  ██▀███    █▓  ▒▓███░  ███▄    █  ",
    "▒████▄    ▒██    ▒ ▓  ██▒ ▓▒▓█   ▀ ▓██   ██ ░██ ▒██▒  ██▒ ██ ▀█   █ ",
    "▒██  ▀█▄  ░ ▓██▄   ▒ ▓██░ ▒░▒███   ▓██ ░▄█  ▒██ ▒██░  ██▒▓██  ▀█ ██▒",
    "░██▄▄▄▄██   ▒   ██▒░ ▓██▓ ░ ▒▓█  ▄ ▒██▀▀█▄  ░██░▒██   ██░▓██▒  ▐▌██▒",
    " ▓█   ▓██▒▒██████▒▒  ▒██▒ ░ ░▒████▒░██▓ ▒██ ░██░  ████▓▒░▒██░   ▓██░",
    " ▒▒   ▓▒█░▒ ▒▓▒ ▒ ░  ▒ ░░   ░░ ▒░ ░░ ▒▓ ░▒▓░░▓  ░ ▒░▒░▒░ ░ ▒░   ▒ ▒ ",
    "  ▒   ▒▒ ░░ ░▒  ░ ░    ░     ░ ░  ░  ░▒ ░ ▒░ ▒ ░  ░ ▒ ▒░ ░ ░░   ░ ▒░",
    "  ░   ▒   ░  ░  ░    ░         ░     ░░   ░  ▒ ░░ ░ ░ ▒     ░   ░ ░ ",
    "      ░  ░      ░              ░  ░   ░      ░      ░ ░           ░ ",
];

fn title_paragraph<'a>() -> Paragraph<'a> {
    let lines = TITLE
        .iter()
        .map(|line| {
            let mut spans = vec![];
            for c in line.chars() {
                // █ ▓ ▒ ░
                if c == '█' {
                    spans.push(Span::styled("█", Color::Rgb(138, 3, 3)));
                } else if c == '▀' {
                    spans.push(Span::styled("▀", Color::Rgb(138, 3, 3)));
                } else if c == '▄' {
                    spans.push(Span::styled("▄", Color::Rgb(138, 3, 3)));
                } else if c == '▌' {
                    spans.push(Span::styled("▌", Color::Rgb(138, 3, 3)));
                } else if c == '▐' {
                    spans.push(Span::styled("▐", Color::Rgb(138, 3, 3)));
                } else if c == '▓' {
                    spans.push(Span::styled("▓", Color::Rgb(118, 3, 3)));
                } else if c == '▒' {
                    spans.push(Span::styled("▒", Color::Rgb(98, 2, 2)));
                } else if c == '░' {
                    spans.push(Span::styled("░", Color::Rgb(78, 0, 0)));
                } else {
                    spans.push(Span::styled(c.to_string(), Color::Rgb(255, 255, 255)));
                }
            }
            Line::from(spans)
        })
        .collect::<Vec<Line>>();
    Paragraph::new(lines).centered()
}

fn format_duration(duration: &Duration) -> String {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    let hours = (duration.as_secs() / 60) / 60;
    let formatted_duration = if hours > 0 {
        format!("{hours}h{minutes:02}m{seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes:02}m{seconds:02}s")
    } else {
        format!("{seconds:02}s")
    };
    formatted_duration.to_string()
}

fn render_header(frame: &mut Frame, game: &Game, hero: &Hero, area: Rect) -> AppResult<()> {
    let number_of_players = game.number_of_players();
    let maze = game.get_maze(hero.maze_id());

    let mut lines = vec![Line::from(format!(
        "There {} {} hero{} in the labyrinth... - Game time {}",
        if number_of_players == 1 { "is" } else { "are" },
        number_of_players,
        if number_of_players == 1 { "" } else { "es" },
        format_duration(&hero.elapsed_duration_from_start()),
    ))];

    let num_minotaurs = game.minotaurs_in_maze(hero.maze_id());
    let (alarm_level, min_distance_squared) = game.alarm_level(&hero.id());
    let radar_power = 16 * 16 / min_distance_squared.max(1);
    let minoradar: String = MINORADAR.iter().take(radar_power).copied().collect();
    let collected = hero.power_ups_collected_in_maze(maze.id());

    lines.push(Line::from(vec![
        Span::styled(format!("{}  ", hero.name()), GameColors::HERO.to_color()),
        Span::raw(format!("Vision {} ", hero.vision())),
        Span::raw(format!("Speed {} ", hero.speed())),
        Span::raw(format!("Memory {} ", hero.memory())),
    ]));

    let mut line = vec![
        Span::raw(format!(
            "Room {}@{:8} - {} minotaur{} - Pass rate {:.2}% - {} power ups left ",
            hero.maze_id() + 1,
            format!("{:?}", hero.position()),
            num_minotaurs,
            if num_minotaurs == 1 { "" } else { "s" },
            maze.success_rate() * 100.0,
            if collected < POWER_UPS_PER_ROOM {
                format!("{}", POWER_UPS_PER_ROOM - collected)
            } else {
                "No".to_string()
            }
        )),
        Span::styled(
            format!("{minoradar:8} "),
            Style::new().fg(alarm_level.rgba().to_color()),
        ),
    ];

    if num_minotaurs > 0 && hero.vision() > 4 {
        line.push(Span::raw(format!(
            "{}",
            (min_distance_squared as f64).sqrt().round() as usize
        )))
    }
    lines.push(Line::from(line));

    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().border_type(BorderType::Double)),
        area,
    );

    Ok(())
}

fn render_sidebar(frame: &mut Frame, game: &Game, hero: &Hero, area: Rect) -> AppResult<()> {
    let split = Layout::vertical([
        Constraint::Min(15),
        Constraint::Max(12),
        Constraint::Max(12),
    ])
    .split(area);

    let lines = vec![
        Line::from("←↑→↓: move"),
        Line::from("'a'/'d': rotate"),
        Line::from("Esc: quit"),
        Line::from(""),
        Line::from(vec![
            Span::styled("██", GameColors::HERO.to_color()),
            Span::raw(format!(" {:12}", "Hero")),
        ]),
        Line::from(vec![
            Span::styled("██", GameColors::OTHER_HERO.to_color()),
            Span::raw(format!(" {:12}", "Other heros")),
        ]),
        Line::from(vec![
            Span::styled("██", GameColors::MINOTAUR.to_color()),
            Span::raw(format!(" {:12}", "Minotaur")),
        ]),
        Line::from(vec![
            Span::styled("██", GameColors::CHASING_MINOTAUR.to_color()),
            Span::raw(format!(" {:12}", "Minotaur (run!)")),
        ]),
        Line::from(vec![
            Span::styled("██", GameColors::POWER_UP.to_color()),
            Span::raw(format!(" {:12}", "Power up")),
        ]),
        Line::from(""),
        Line::from("Run from the minotaurs".to_string()),
        Line::from("and try to get as far".to_string()),
        Line::from("as possible.".to_string()),
    ];

    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().border_set(border::DOUBLE)),
        split[0],
    );
    let lines = game
        .top_heros()
        .iter()
        .take(10)
        .map(|(id, name, maze_id, duration)| {
            let record = if *maze_id < MAX_MAZE_ID {
                format!("r{}", maze_id + 1,)
            } else {
                format_duration(duration)
            };
            Line::from(Span::styled(
                format!("{name:<NAME_LENGTH$} {record}"),
                if game.get_hero(id).is_some() {
                    if *id == hero.id() {
                        Style::new().fg(GameColors::HERO.to_color())
                    } else {
                        Style::new().fg(GameColors::OTHER_HERO.to_color())
                    }
                } else {
                    Style::new()
                },
            ))
        })
        .collect_vec();

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title("Top Heros")
                .border_set(border::DOUBLE),
        ),
        split[1],
    );

    let lines = game
        .top_minotaurs()
        .iter()
        .take(10)
        .map(|(_, name, maze_id, kills)| {
            Line::from(format!(
                "{:<NAME_LENGTH$} k{:<3} r{}",
                name,
                kills,
                maze_id + 1
            ))
        })
        .collect_vec();

    frame.render_widget(
        Paragraph::new(lines).block(
            Block::bordered()
                .title("Top Minotaurs")
                .border_set(border::DOUBLE),
        ),
        split[2],
    );

    Ok(())
}

pub fn render(
    frame: &mut Frame,
    game: &Game,
    player_id: PlayerId,
    start_instant: Instant,
) -> AppResult<()> {
    let screen_area = screen_area(frame.area());

    if start_instant.elapsed() < Duration::from_millis(1500) {
        frame.render_widget(title_paragraph(), screen_area.inner(Margin::new(4, 2)));
        return Ok(());
    }

    if screen_area.width < UI_SCREEN_SIZE.0 || screen_area.height < UI_SCREEN_SIZE.1 {
        frame.render_widget(
            Paragraph::new(format!(
                " Frame size {}x{} is smaller than the minimum size {}x{}.\nPlease resize it or exit with 'q'.",
                screen_area.width,
                screen_area.height,
                UI_SCREEN_SIZE.0,
                UI_SCREEN_SIZE.1
            ))
            .centered()
            .wrap(Wrap { trim: true }),
            screen_area,
        );
        return Ok(());
    }

    let hero = if let Some(hero) = game.get_hero(&player_id) {
        hero
    } else {
        return Err(anyhow!("Missing hero {player_id}"));
    };

    let h_split =
        Layout::horizontal([Constraint::Fill(1), Constraint::Length(24)]).split(screen_area);
    render_sidebar(frame, game, hero, h_split[1])?;

    let v_split = Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]).split(h_split[0]);
    render_header(frame, game, hero, v_split[0])?;

    let image = game.draw(player_id)?;

    // Override empty positions.
    let override_positions = game.image_char_overrides(player_id, &image)?;

    frame.render_widget(
        Paragraph::new(img_to_lines(
            &image,
            override_positions,
            Maze::background_color(),
        ))
        .block(Block::bordered().border_type(BorderType::Double)),
        v_split[1],
    );

    if hero.is_dead() {
        let width = 32;
        let height = 6;
        let popup = Rect::new(
            v_split[1].x + (v_split[1].width.saturating_sub(width)) / 2,
            v_split[1].y + (v_split[1].height.saturating_sub(height)) / 2,
            width,
            height,
        );

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(hero.name().to_string()),
                Line::from(format!("died while exploring room {}", hero.maze_id() + 1)),
            ])
            .centered()
            .set_style(Style::default().fg(Color::Black).bg(Color::Red))
            .block(Block::bordered().border_type(BorderType::QuadrantOutside)),
            popup,
        );
    } else if let Some(duration) = hero.has_won().as_ref() {
        let width = 32;
        let height = 6;
        let popup = Rect::new(
            v_split[1].x + (v_split[1].width.saturating_sub(width)) / 2,
            v_split[1].y + (v_split[1].height.saturating_sub(height)) / 2,
            width,
            height,
        );

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(hero.name().to_string()),
                Line::from("exited the labyrinth in".to_string()),
                Line::from(format_duration(duration)),
            ])
            .centered()
            .set_style(Style::default().fg(Color::Black).bg(Color::LightGreen))
            .block(Block::bordered().border_type(BorderType::QuadrantOutside)),
            popup,
        );
    }

    Ok(())
}

fn screen_area(frame_area: Rect) -> Rect {
    // If area is bigger than UI_SCREEN_SIZE, use a centered rect of the correct size.
    let frame_width = frame_area.width;
    let frame_height = frame_area.height;
    let (target_width, target_height) = UI_SCREEN_SIZE;
    Rect::new(
        frame_width.saturating_sub(target_width) / 2,
        frame_height.saturating_sub(target_height) / 2,
        target_width.min(frame_width),
        target_height.min(frame_height),
    )
}
