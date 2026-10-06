//! The in-car dashboard: a grid of panels under a one-line status bar.
//!
//! To change what's on screen, edit [`Dashboard::new`]. To add a new kind of
//! display, implement [`Panel`] in `panels/`.

mod big_number;
mod chart;
mod panels;
mod scale;
mod status;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::signal::Signal;
use crate::telemetry::Telemetry;
use panels::{Placeholder, RpmPanel, SpeedPanel, TempsPanel};

pub trait Panel {
    fn render(&self, frame: &mut Frame, area: Rect, telemetry: &Telemetry);
}

struct PanelRow {
    height: Constraint,
    panels: Vec<(Constraint, Box<dyn Panel>)>,
}

pub struct Dashboard {
    rows: Vec<PanelRow>,
}

impl Dashboard {
    pub fn new() -> Self {
        use Constraint::Percentage;
        Self {
            rows: vec![
                PanelRow {
                    height: Percentage(60),
                    panels: vec![
                        (Percentage(40), Box::new(SpeedPanel)),
                        (Percentage(60), Box::new(RpmPanel)),
                    ],
                },
                PanelRow {
                    height: Percentage(40),
                    panels: vec![
                        (
                            Percentage(50),
                            Box::new(TempsPanel::new(vec![Signal::CoolantTempC, Signal::IntakeAirTempC])),
                        ),
                        (Percentage(50), Box::new(Placeholder::new("TBD"))),
                    ],
                },
            ],
        }
    }

    pub fn draw(&self, frame: &mut Frame, telemetry: &Telemetry) {
        let [status_area, body] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(frame.area());
        frame.render_widget(status::line(telemetry), status_area);

        let row_areas = Layout::vertical(self.rows.iter().map(|r| r.height)).split(body);
        for (row, &row_area) in self.rows.iter().zip(row_areas.iter()) {
            let cells = Layout::horizontal(row.panels.iter().map(|(width, _)| *width)).split(row_area);
            for ((_, panel), &cell) in row.panels.iter().zip(cells.iter()) {
                panel.render(frame, cell, telemetry);
            }
        }
    }
}
