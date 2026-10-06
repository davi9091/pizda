mod placeholder;
mod rpm;
mod speed;
mod temps;

pub use placeholder::Placeholder;
pub use rpm::RpmPanel;
pub use speed::SpeedPanel;
pub use temps::TempsPanel;

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Block;

/// Draw the standard panel frame and return the area inside it.
fn framed(frame: &mut Frame, area: Rect, title: &str) -> Rect {
    let block = Block::bordered().title(format!(" {title} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}
