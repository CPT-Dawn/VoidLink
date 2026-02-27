//! Help overlay — keybinding reference.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::theme;

/// Render the help overlay.
pub fn render(frame: &mut Frame, _app: &App) {
    let area = centered_rect(60, 70, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" 󰋖 Keybindings ", theme::title()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::deep_purple()));

    let keybindings = vec![
        (
            "Navigation",
            vec![
                ("j / ↓", "Move cursor down"),
                ("k / ↑", "Move cursor up"),
                ("g", "Jump to top"),
                ("G", "Jump to bottom"),
            ],
        ),
        (
            "Device Actions",
            vec![
                ("Enter", "Connect / Disconnect (toggle)"),
                ("p", "Pair with device"),
                ("t", "Toggle trusted"),
                ("d", "Disconnect device"),
                ("r", "Remove / forget device"),
                ("R", "Refresh device info"),
            ],
        ),
        (
            "Adapter",
            vec![("a", "Toggle adapter power"), ("s", "Toggle scanning")],
        ),
        (
            "Other",
            vec![
                ("/", "Search devices"),
                ("?", "Toggle this help"),
                ("q", "Quit VoidLink"),
                ("Esc", "Dismiss popup / exit search"),
            ],
        ),
    ];

    let mut lines = vec![Line::from("")];

    for (section, bindings) in &keybindings {
        lines.push(Line::from(Span::styled(
            format!("  ── {section} ──"),
            Style::default()
                .fg(theme::cyan())
                .add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(""));

        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("    {key:<12}"),
                    Style::default()
                        .fg(theme::deep_purple())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(*desc, theme::list_item()),
            ]));
        }
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "  Press ESC or ? to close",
        theme::dim(),
    )));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Compute a centered rectangle.
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
