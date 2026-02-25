//! Popup / dialog overlays with sliding entrance animations.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
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
            render_sliding_dialog(
                frame,
                "  Error ",
                message,
                theme::error(),
                theme::DAWN_RED,
                *slide,
            );
        }
        Popup::ConnectionResult {
            success, message, slide, ..
        } => {
            let (title, style, color) = if *success {
                (" 󰂱 Connected ", theme::connected(), theme::CYAN)
            } else {
                ("  Failed ", theme::error(), theme::DAWN_RED)
            };
            render_sliding_dialog(frame, title, message, style, color, *slide);
        }
        Popup::PinDisplay { pin, slide, .. } => {
            let area = centered_rect(40, 7, frame.area());
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

            let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: true });
            frame.render_widget(paragraph, animated);
        }
        Popup::Help => {
            super::help::render(frame, app);
        }
    }
}

/// Render a generic sliding dialog box.
fn render_sliding_dialog(
    frame: &mut Frame,
    title: &str,
    message: &str,
    title_style: Style,
    border_color: ratatui::style::Color,
    slide: f32,
) {
    let area = centered_rect(50, 5, frame.area());
    let animated = slide_from_top(area, slide);

    frame.render_widget(Clear, animated);

    let block = Block::default()
        .title(Span::styled(title, title_style))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {message}"),
            theme::list_item(),
        )),
        Line::from(Span::styled("  Press ESC to dismiss", theme::dim())),
    ];

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, animated);
}

/// Compute a centered rectangle of `percent_x`% width and `height` lines.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height.min(100)) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - height.min(100)) / 2),
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
