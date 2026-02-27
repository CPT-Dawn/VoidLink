//! Adapter status bar at the top of the screen.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let adapter = &app.adapter;

    let mut spans = vec![
        Span::styled(" 󰂯 VoidLink ", theme::title()),
        Span::styled("│ ", theme::dim()),
    ];

    // Adapter name & address.
    let addr_str = adapter
        .address
        .map(|a| a.to_string())
        .unwrap_or_else(|| "??:??:??:??:??:??".into());
    spans.push(Span::styled(
        format!("{} [{}] ", adapter.name, addr_str),
        theme::list_item(),
    ));

    spans.push(Span::styled("│ ", theme::dim()));

    // Power state.
    if adapter.powered {
        spans.push(Span::styled("⏻ ON ", theme::connected()));
    } else {
        spans.push(Span::styled("⏻ OFF ", theme::error()));
    }

    spans.push(Span::styled("│ ", theme::dim()));

    // Scanning state with animated spinner.
    if app.scanning {
        let frame_char = theme::spinner_frame(app.tick_count);
        spans.push(Span::styled(
            format!("{frame_char} Scanning "),
            ratatui::style::Style::default()
                .fg(theme::scanning_pulse())
                .add_modifier(ratatui::style::Modifier::BOLD),
        ));
    } else {
        spans.push(Span::styled("  Idle ", theme::dim()));
    }

    // Search indicator.
    if app.input_mode == InputMode::Search {
        spans.push(Span::styled("│ ", theme::dim()));
        spans.push(Span::styled(
            format!("/ {}", app.search_query),
            ratatui::style::Style::default()
                .fg(theme::cyan())
                .add_modifier(ratatui::style::Modifier::ITALIC),
        ));
        spans.push(Span::styled("█", theme::connected())); // cursor
    }

    // Device count (right-aligned via padding — simple approach).
    let device_count = app.filtered_devices().len();
    spans.push(Span::styled("│ ", theme::dim()));
    spans.push(Span::styled(
        format!("{device_count} devices"),
        theme::dim(),
    ));

    let line = Line::from(spans);
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(theme::border_active());

    let paragraph = Paragraph::new(line).block(block);
    frame.render_widget(paragraph, area);
}
