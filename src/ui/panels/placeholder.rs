use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

use super::framed;
use crate::telemetry::Telemetry;
use crate::ui::Panel;

/// Reserves a slot in the layout for a panel that doesn't exist yet.
pub struct Placeholder {
    title: &'static str,
}

impl Placeholder {
    pub fn new(title: &'static str) -> Self {
        Self { title }
    }
}

impl Panel for Placeholder {
    fn render(&self, frame: &mut Frame, area: Rect, _telemetry: &Telemetry) {
        let inner = framed(frame, area, self.title);
        frame.render_widget(Paragraph::new("—").style(Style::new().fg(Color::DarkGray)).centered(), inner);
    }
}
