//! One-line header: brand, adapter state on the left; counts and sort on the right.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, BtStatus};
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let sep = || Span::styled("  ·  ", theme::fg(theme::edge()));

    let mut left = vec![
        Span::styled(
            format!(" {} VoidLink ", theme::logo()),
            theme::bold(theme::accent()).add_modifier(Modifier::REVERSED),
        ),
        Span::raw("  "),
    ];

    match &app.status {
        BtStatus::Starting => {
            left.push(Span::styled(
                format!("{} Connecting to BlueZ…", theme::spinner(app.elapsed_ms())),
                theme::dim(),
            ));
        }
        BtStatus::Unavailable(_) => {
            left.push(Span::styled("● Bluetooth unavailable", theme::error()));
        }
        BtStatus::Ready => {
            let name = if app.adapter.alias.is_empty() {
                app.adapter.name.as_str()
            } else {
                app.adapter.alias.as_str()
            };
            left.push(Span::styled(name.to_string(), theme::body()));
            left.push(sep());
            if app.adapter.powered {
                left.push(Span::styled(
                    format!("{} On", theme::power()),
                    theme::bold(theme::green()),
                ));
            } else {
                left.push(Span::styled(
                    format!("{} Off", theme::power()),
                    theme::bold(theme::red()),
                ));
            }
            if app.scanning {
                left.push(sep());
                left.push(Span::styled(
                    format!("{} Scanning", theme::spinner(app.elapsed_ms())),
                    theme::bold(theme::pulse()),
                ));
            }
        }
    }

    let mut right = Vec::new();
    if app.filter_active() {
        right.push(Span::styled(
            format!("{} {}", theme::search_icon(), app.search_query),
            theme::bold(theme::accent2()),
        ));
        right.push(sep());
    }
    if app.is_ready() {
        let n = app.visible_count();
        right.push(Span::styled(
            format!("{n} device{}", if n == 1 { "" } else { "s" }),
            theme::dim(),
        ));
        right.push(sep());
        right.push(Span::styled(
            format!("{} {}", theme::sort_glyph(), app.sort_mode.label()),
            theme::dim(),
        ));
        right.push(Span::raw(" "));
    }

    let left = Line::from(left);
    let right = Line::from(right);
    // Drop the right side rather than overlap on narrow terminals.
    if usize::from(area.width) > left.width() + right.width() + 2 {
        frame.render_widget(Paragraph::new(right).alignment(Alignment::Right), area);
    }
    let left_area = Rect {
        width: u16::try_from(left.width())
            .unwrap_or(u16::MAX)
            .min(area.width),
        ..area
    };
    frame.render_widget(Paragraph::new(left), left_area);
}
