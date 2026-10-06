use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::telemetry::{SourceStatus, Telemetry};

/// One coloured dot per source, followed by its detail or error.
pub fn line(telemetry: &Telemetry) -> Line<'static> {
    let mut spans = Vec::new();
    for (name, status) in telemetry.sources() {
        let (color, detail) = match status {
            SourceStatus::Connecting => (Color::Yellow, None),
            SourceStatus::Connected(detail) => (Color::Green, Some(detail.clone())),
            SourceStatus::Error(err) => (Color::Red, Some(err.clone())),
        };
        spans.push(Span::styled("● ", Style::new().fg(color)));
        spans.push(Span::raw(name));
        if let Some(detail) = detail {
            spans.push(Span::styled(format!(" {detail}"), Style::new().fg(Color::DarkGray)));
        }
        spans.push(Span::raw("  "));
    }
    Line::from(spans)
}
