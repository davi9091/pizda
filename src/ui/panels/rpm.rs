use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::Gauge;

use super::framed;
use crate::signal::Signal;
use crate::telemetry::Telemetry;
use crate::ui::scale::{Zone, format_value, scale};
use crate::ui::{Panel, chart};

const SIGNAL: Signal = Signal::Rpm;
const WINDOW: Duration = Duration::from_secs(30);

/// Rev bar that changes colour approaching redline, with a history graph.
pub struct RpmPanel;

impl Panel for RpmPanel {
    fn render(&self, frame: &mut Frame, area: Rect, telemetry: &Telemetry) {
        let inner = framed(frame, area, "RPM");
        let [bar, graph] = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(inner);

        let scale = scale(SIGNAL);
        let reading = telemetry.latest(SIGNAL).filter(|r| !r.is_stale());
        let value = reading.map_or(0.0, |r| r.value);
        let color = match scale.zone(value) {
            Zone::Normal => Color::Cyan,
            zone => zone.color(),
        };
        frame.render_widget(
            Gauge::default()
                .ratio(scale.ratio(value))
                .label(format_value(SIGNAL, reading))
                .gauge_style(Style::new().fg(color))
                .use_unicode(true),
            bar,
        );
        chart::render_history(frame, graph, telemetry, SIGNAL, WINDOW, Color::Cyan);
    }
}
