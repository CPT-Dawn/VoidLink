//! Help overlay — keybinding reference.

use ratatui::layout::Rect;
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
                ("A", "Rename device alias"),
            ],
        ),
        (
            "Adapter",
            vec![
                ("a", "Toggle adapter power"),
                ("s", "Toggle scanning"),
                ("S", "Cycle sort mode"),
            ],
        ),
        (
            "Other",
            vec![
                ("/", "Search (regex: start with /)"),
                ("?", "Toggle this help"),
                ("q", "Quit VoidLink"),
                ("Esc", "Dismiss popup / exit mode"),
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

/// Compute a centered rectangle (safe — no raw indexing).
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let width = (area.width as u32 * percent_x.min(100) as u32 / 100) as u16;
    let height = (area.height as u32 * percent_y.min(100) as u32 / 100) as u16;
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect { x, y, width, height }
}
