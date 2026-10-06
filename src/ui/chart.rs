use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::widgets::{Axis, Chart, Dataset, GraphType};

use super::scale::scale;
use crate::signal::Signal;
use crate::telemetry::Telemetry;

/// Line graph of a signal over the last `window`, newest on the right.
pub fn render_history(
    frame: &mut Frame,
    area: Rect,
    telemetry: &Telemetry,
    signal: Signal,
    window: Duration,
    color: Color,
) {
    let scale = scale(signal);
    let now = Instant::now();
    let points: Vec<(f64, f64)> = telemetry
        .history(signal)
        .filter_map(|r| {
            let age = now.saturating_duration_since(r.at);
            (age <= window).then(|| (-age.as_secs_f64(), r.value.clamp(scale.min, scale.max)))
        })
        .collect();

    let dataset = Dataset::default()
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::new().fg(color))
        .data(&points);
    let axis_style = Style::new().fg(Color::DarkGray);
    let chart = Chart::new(vec![dataset])
        .x_axis(Axis::default().bounds([-window.as_secs_f64(), 0.0]))
        .y_axis(
            Axis::default()
                .bounds([scale.min, scale.max])
                .labels([format!("{:.0}", scale.min), format!("{:.0}", scale.max)])
                .style(axis_style),
        );
    frame.render_widget(chart, area);
}
