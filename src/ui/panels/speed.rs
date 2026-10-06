use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;

use super::framed;
use crate::signal::Signal;
use crate::telemetry::Telemetry;
use crate::ui::scale::{format_value, value_style};
use crate::ui::{Panel, big_number, chart};

const SIGNAL: Signal = Signal::SpeedKph;
const WINDOW: Duration = Duration::from_secs(60);

/// Big speed readout with a history graph underneath.
pub struct SpeedPanel;

impl Panel for SpeedPanel {
    fn render(&self, frame: &mut Frame, area: Rect, telemetry: &Telemetry) {
        let inner = framed(frame, area, "Speed");
        let [number, unit, graph] = Layout::vertical([
            Constraint::Length(big_number::HEIGHT),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .areas(inner);

        let reading = telemetry.latest(SIGNAL);
        big_number::render(frame, number, &format_value(SIGNAL, reading), value_style(SIGNAL, reading));
        frame.render_widget(
            Paragraph::new(SIGNAL.unit()).style(Style::new().fg(Color::DarkGray)).centered(),
            unit,
        );
        chart::render_history(frame, graph, telemetry, SIGNAL, WINDOW, Color::Cyan);
    }
}
