//! Animated loading spinner widget.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme;

/// Render a scanning spinner at the given area.
#[allow(dead_code)]
pub fn render(frame: &mut Frame, tick: u64, area: Rect, label: &str) {
    let spinner_char = theme::spinner_frame(tick);
    let line = Line::from(vec![
        Span::styled(
            format!(" {spinner_char} "),
            Style::default()
                .fg(theme::SCANNING_PULSE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(label, Style::default().fg(theme::CYAN)),
    ]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
