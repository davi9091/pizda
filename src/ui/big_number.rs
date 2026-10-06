//! Large block-character digits, readable at a glance while driving.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

pub const HEIGHT: u16 = 5;
const GLYPH_WIDTH: u16 = 3;

fn glyph(c: char) -> Option<[&'static str; HEIGHT as usize]> {
    Some(match c {
        '0' => ["███", "█ █", "█ █", "█ █", "███"],
        '1' => [" █ ", "██ ", " █ ", " █ ", "███"],
        '2' => ["███", "  █", "███", "█  ", "███"],
        '3' => ["███", "  █", "███", "  █", "███"],
        '4' => ["█ █", "█ █", "███", "  █", "  █"],
        '5' => ["███", "█  ", "███", "  █", "███"],
        '6' => ["███", "█  ", "███", "█ █", "███"],
        '7' => ["███", "  █", "  █", "  █", "  █"],
        '8' => ["███", "█ █", "███", "█ █", "███"],
        '9' => ["███", "█ █", "███", "  █", "███"],
        '-' => ["   ", "   ", "███", "   ", "   "],
        ' ' => ["   ", "   ", "   ", "   ", "   "],
        _ => return None,
    })
}

/// Render `text` centred in `area` in big digits, falling back to normal
/// text when the area is too small or `text` has unsupported characters.
pub fn render(frame: &mut Frame, area: Rect, text: &str, style: Style) {
    let glyphs: Option<Vec<_>> = text.chars().map(glyph).collect();
    let chars = text.chars().count() as u16;
    let width = chars * (GLYPH_WIDTH + 1) - 1;

    let lines: Vec<Line> = match glyphs {
        Some(glyphs) if area.height >= HEIGHT && area.width >= width => (0..HEIGHT as usize)
            .map(|row| Line::from(glyphs.iter().map(|g| g[row]).collect::<Vec<_>>().join(" ")))
            .collect(),
        _ => vec![Line::from(text.to_string())],
    };
    frame.render_widget(Paragraph::new(lines).style(style).centered(), area);
}
