use image::{Pixel, Rgba, RgbaImage};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::collections::HashMap;

pub fn img_to_lines<'a>(
    img: &RgbaImage,
    image_char_overrides: HashMap<(u32, u32), char>,
    background_color: Rgba<u8>,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = vec![];
    let width = img.width();
    let height = img.height();

    for y in (0..height - 1).step_by(2) {
        let mut line: Vec<Span> = vec![];

        'pixels: for x in 0..width {
            let &top_pixel = img.get_pixel(x, y);
            let &btm_pixel = img.get_pixel(x, y + 1);

            // check for overrides.
            // Maze valid, visible, not occupied positions (alpha < 255)
            if image_char_overrides.contains_key(&(x, y))
                && image_char_overrides.contains_key(&(x, y + 1))
            {
                let [_, _, _, ta] = top_pixel.0;
                let [_, _, _, ba] = btm_pixel.0;
                // This is a hack to only override empty cells,
                // since 'stuff' is always printed with alpha=255.
                if ta < 255 && ba < 255 {
                    let override_char = image_char_overrides.get(&(x, y)).unwrap();
                    line.push(Span::styled(
                        override_char.to_string(),
                        Style::default().fg(Color::Rgb(ta, ta, ta)),
                    ));
                    continue 'pixels;
                }
            }

            // both pixels are transparent
            if top_pixel.is_transparent(background_color)
                && btm_pixel.is_transparent(background_color)
            {
                line.push(Span::raw(" "));
                continue;
            }

            // render top pixel
            if !top_pixel.is_transparent(background_color)
                && btm_pixel.is_transparent(background_color)
            {
                let color = top_pixel.to_color();
                line.push(Span::styled("▀", Style::default().fg(color)));
                continue;
            }

            // render bottom pixel
            if top_pixel.is_transparent(background_color)
                && !btm_pixel.is_transparent(background_color)
            {
                let color = btm_pixel.to_color();
                line.push(Span::styled("▄", Style::default().fg(color)));
                continue;
            }

            // render both pixels
            let fg_color = top_pixel.to_color();
            let bg_color = btm_pixel.to_color();
            line.push(Span::styled(
                "▀",
                Style::default().fg(fg_color).bg(bg_color),
            ));
        }
        lines.push(Line::from(line));
    }
    // append last line if height is odd
    if height % 2 == 1 {
        let mut line: Vec<Span> = vec![];
        for x in 0..width {
            let top_pixel = img.get_pixel(x, height - 1);
            if top_pixel[3] == 0 {
                line.push(Span::raw(" "));
                continue;
            }
            let [r, g, b, _] = top_pixel.0;
            let color = Color::Rgb(r, g, b);
            line.push(Span::styled("▀", Style::default().fg(color)));
        }
        lines.push(Line::from(line));
    }

    lines
}

pub trait RataColor {
    fn to_color(&self) -> Color;
    fn is_transparent(&self, background_color: Self) -> bool;
}

impl RataColor for Rgba<u8> {
    fn to_color(&self) -> Color {
        let [r, g, b, a] = self.0;
        let alpha = a as f32 / 255.0;
        let r = (r as f32 * alpha) as u8;
        let g = (g as f32 * alpha) as u8;
        let b = (b as f32 * alpha) as u8;

        Color::Rgb(r, g, b)
    }

    fn is_transparent(&self, background_color: Self) -> bool {
        self[3] == 0 || self.to_rgb() == background_color.to_rgb()
    }
}
