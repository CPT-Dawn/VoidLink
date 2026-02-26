//! Popup / dialog overlays with sliding entrance animations.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{App, Popup};
use crate::theme;

/// Render the active popup overlay.
pub fn render(frame: &mut Frame, app: &App, popup: &Popup) {
    match popup {
        Popup::Error { message, slide } => {
            render_status_dialog(
                frame,
                "  Error ",
                message,
                theme::error(),
                theme::DAWN_RED,
                *slide,
            );
        }
        Popup::ConnectionResult {
            success,
            message,
            slide,
            ..
        } => {
            let (title, style, color) = if *success {
                (" 󰂱 Connected ", theme::connected(), theme::CYAN)
            } else {
                (" 󰅙 Connection Failed ", theme::error(), theme::DAWN_RED)
            };
            render_status_dialog(frame, title, message, style, color, *slide);
        }
        Popup::PinDisplay { pin, slide, .. } => {
            let area = centered_rect_percent(40, 7, frame.area());
            let animated = slide_from_top(area, *slide);

            frame.render_widget(Clear, animated);
            let block = Block::default()
                .title(Span::styled(" 󰌾 Pairing PIN ", theme::title()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::DEEP_PURPLE));

            let content = vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("  PIN: {pin}"),
                    Style::default()
                        .fg(theme::CYAN)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Confirm on your device. Press ESC to dismiss.",
                    theme::dim(),
                )),
            ];

            let paragraph = Paragraph::new(content)
                .block(block)
                .wrap(Wrap { trim: true });
            frame.render_widget(paragraph, animated);
        }
        Popup::Help => {
            super::help::render(frame, app);
        }
    }
}

/// Render a generic sliding dialog box.
fn render_status_dialog(
    frame: &mut Frame,
    title: &str,
    message: &str,
    title_style: Style,
    border_color: ratatui::style::Color,
    slide: f32,
) {
    let viewport = frame.area();
    let max_width = viewport.width.saturating_sub(2).max(24);
    let preferred_width = ((viewport.width as u32 * 68) / 100) as u16;
    let width = preferred_width.clamp(24, max_width);
    let message_width = width.saturating_sub(4).max(1);
    let message_lines = wrapped_line_count(message, message_width);
    let max_height = viewport.height.saturating_sub(2).max(4);
    let height = (message_lines + 4).clamp(4, max_height);

    let area = centered_rect(width, height, viewport);
    let animated = slide_from_top(area, slide);

    frame.render_widget(Clear, animated);

    let block = Block::default()
        .title(Span::styled(title, title_style))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let content = vec![
        Line::from(Span::raw("")),
        Line::from(Span::styled(format!("  {message}"), theme::list_item())),
        Line::from(Span::raw("")),
        Line::from(vec![
            Span::styled(
                "  Esc ",
                Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("dismiss", theme::dim()),
        ]),
    ];

    let paragraph = Paragraph::new(content)
        .block(block)
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, animated);
}

/// Compute a centered rectangle with fixed width/height in terminal cells.
fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width).max(1);
    let height = height.min(area.height).max(1);
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Compute a centered rectangle where width is a percentage of available area.
fn centered_rect_percent(percent_x: u16, height: u16, area: Rect) -> Rect {
    let percent_x = percent_x.clamp(10, 100);
    let width = ((area.width as u32 * percent_x as u32) / 100) as u16;
    centered_rect(width, height, area)
}

/// Rough wrapped line estimator for status messages.
fn wrapped_line_count(message: &str, content_width: u16) -> u16 {
    let width = usize::from(content_width.max(1));
    let mut lines = 0usize;

    for segment in message.split('\n') {
        let len = segment.chars().count();
        let wrapped = len.div_ceil(width).max(1);
        lines += wrapped;
    }

    lines as u16
}

/// Apply a slide-from-top animation to a rect. `progress` is 0.0 → 1.0.
fn slide_from_top(target: Rect, progress: f32) -> Rect {
    let progress = progress.clamp(0.0, 1.0);
    let offset = ((1.0 - progress) * target.y as f32) as u16;
    Rect {
        x: target.x,
        y: target.y.saturating_sub(offset),
        width: target.width,
        height: target.height,
    }
}
