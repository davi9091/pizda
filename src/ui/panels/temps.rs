use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols;
use ratatui::text::Line;
use ratatui::widgets::{LineGauge, Paragraph};

use super::framed;
use crate::signal::Signal;
use crate::telemetry::Telemetry;
use crate::ui::Panel;
use crate::ui::scale::{format_value, scale, value_style};

/// One row per temperature: label, value, and a bar coloured by zone.
pub struct TempsPanel {
    signals: Vec<Signal>,
}

impl TempsPanel {
    pub fn new(signals: Vec<Signal>) -> Self {
        Self { signals }
    }
}

impl Panel for TempsPanel {
    fn render(&self, frame: &mut Frame, area: Rect, telemetry: &Telemetry) {
        let inner = framed(frame, area, "Temps");
        // One line per row plus a blank line between rows.
        let rows = Layout::vertical(self.signals.iter().map(|_| Constraint::Length(2))).split(inner);

        for (&signal, row) in self.signals.iter().zip(rows.iter()) {
            let [label, value, bar] = Layout::horizontal([
                Constraint::Length(16),
                Constraint::Length(8),
                Constraint::Min(0),
            ])
            .spacing(1)
            .areas(Rect { height: 1, ..*row });

            let reading = telemetry.latest(signal);
            let style = value_style(signal, reading);
            frame.render_widget(Paragraph::new(signal.label()), label);
            frame.render_widget(
                Line::from(format!("{} {}", format_value(signal, reading), signal.unit()))
                    .style(style)
                    .right_aligned(),
                value,
            );
            frame.render_widget(
                LineGauge::default()
                    .ratio(reading.map_or(0.0, |r| scale(signal).ratio(r.value)))
                    .label("")
                    .filled_symbol(symbols::line::THICK_HORIZONTAL)
                    .unfilled_symbol(symbols::line::THICK_HORIZONTAL)
                    .filled_style(style)
                    .unfilled_style(Style::new().fg(Color::DarkGray)),
                bar,
            );
        }
    }
}
