//! Rendering. `render` lays out the frame and dispatches to the widgets:
//!
//! ```text
//!  ᛒ VoidLink  hci0 · ● On · ⠋ Scanning              5 devices · ⇅ Smart
//! ╭ Devices ───────────────────────────╮ ╭ Details ──────────────────────╮
//! │ MY DEVICES ─────────────────────── │ │ ...                           │
//! ╰────────────────────────────────────╯ ╰───────────────────────────────╯
//!   Enter Connect   s Scan   / Search   p Pair  …            ? Help  q Quit
//! ```
//!
//! Narrow terminals stack the details under the list, or hide them.

pub mod detail_panel;
pub mod device_list;
pub mod help;
pub mod key_bar;
pub mod popup;
pub mod status_bar;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};
use ratatui::Frame;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, BtStatus};
use crate::{config, theme};

/// Smallest terminal VoidLink will draw into.
const MIN_WIDTH: u16 = 44;
const MIN_HEIGHT: u16 = 12;

pub fn render(frame: &mut Frame, app: &mut App) {
    let full = frame.area();
    if full.width < MIN_WIDTH || full.height < MIN_HEIGHT {
        render_too_small(frame, full);
        return;
    }

    let area = if full.width >= 80 {
        full.inner(ratatui::layout::Margin {
            horizontal: 2,
            vertical: 1,
        })
    } else {
        full.inner(ratatui::layout::Margin {
            horizontal: 1,
            vertical: 0,
        })
    };

    let [header, _, body, _, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    status_bar::render(frame, app, header);
    key_bar::render(frame, app, footer);

    if let BtStatus::Unavailable(msg) = &app.status {
        render_unavailable(frame, msg, body);
    } else if body.width >= 96 {
        let pct = config::get().general.device_list_percent;
        let [list, _, detail] = Layout::horizontal([
            Constraint::Percentage(pct),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(body);
        device_list::render(frame, app, list);
        detail_panel::render(frame, app, detail);
    } else if body.height >= 26 {
        let [list, detail] =
            Layout::vertical([Constraint::Min(6), Constraint::Length(13)]).areas(body);
        device_list::render(frame, app, list);
        detail_panel::render(frame, app, detail);
    } else {
        device_list::render(frame, app, body);
    }

    popup::render_toasts(frame, app, body);
    popup::render_modal(frame, app);
}

fn render_too_small(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(Span::styled("Terminal too small", theme::title())),
        Line::from(Span::styled(
            format!(
                "Need {MIN_WIDTH}×{MIN_HEIGHT}, have {}×{}",
                area.width, area.height
            ),
            theme::dim(),
        )),
    ];
    let y = area.y + area.height.saturating_sub(2) / 2;
    let rect = Rect {
        y,
        height: 2.min(area.height),
        ..area
    };
    frame.render_widget(Paragraph::new(text).alignment(Alignment::Center), rect);
}

fn render_unavailable(frame: &mut Frame, message: &str, area: Rect) {
    let block = panel(" Bluetooth unavailable ", true).border_style(theme::fg(theme::red()));
    frame.render_widget(block, area);

    let card_w = area.width.saturating_sub(8).min(72);
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{}  Bluetooth isn't available", theme::logo()),
            theme::bold(theme::red()),
        )),
        Line::raw(""),
    ];
    lines.extend(message.split(". ").map(|s| {
        Line::from(Span::styled(
            s.trim_end_matches('.').to_string(),
            theme::body(),
        ))
    }));
    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled(
        "VoidLink reconnects automatically as soon as it's back.",
        theme::dim(),
    )));
    let height = wrapped_height(&lines, card_w);
    let rect = centered(card_w, height, area);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        rect,
    );
}

// ─── Shared helpers ─────────────────────────────────────────────────────────

/// A rounded, padded panel with a title.
pub fn panel(title: &str, focused: bool) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            theme::border_focus()
        } else {
            theme::border()
        })
        .title(Span::styled(title, theme::title()))
        .padding(Padding::horizontal(1))
}

/// Draw a centered modal frame and return its inner area.
pub fn modal_frame(
    frame: &mut Frame,
    title: &str,
    width: u16,
    height: u16,
    color: ratatui::style::Color,
) -> Rect {
    let area = centered(width, height, frame.area());
    frame.render_widget(Clear, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::fg(color))
        .title(Span::styled(title, theme::bold(color)))
        .padding(Padding::new(2, 2, 1, 0));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner
}

/// A `width`×`height` rect centered in `area` (clamped to fit).
pub fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

/// Lines needed to show `lines` wrapped at `width`.
pub fn wrapped_height(lines: &[Line], width: u16) -> u16 {
    let width = usize::from(width.max(1));
    let total: usize = lines.iter().map(|l| l.width().div_ceil(width).max(1)).sum();
    u16::try_from(total).unwrap_or(u16::MAX)
}

/// Truncate to a display width, adding `…` when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.width() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in s.chars() {
        let w = c.width().unwrap_or(0);
        if used + w + 1 > max {
            break;
        }
        out.push(c);
        used += w;
    }
    out.push('…');
    out
}

/// Pad with spaces to a display width.
pub fn pad(s: &str, width: usize) -> String {
    let w = s.width();
    if w >= width {
        s.to_string()
    } else {
        format!("{s}{}", " ".repeat(width - w))
    }
}

/// `key` + `label` pair as styled spans, e.g. "Enter Connect".
pub fn hint(key: &str, label: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(key.to_string(), theme::key()),
        Span::styled(format!(" {label}"), theme::dim()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Modal;
    use crate::bluetooth::types::{AdapterInfo, Address, BtEvent, DeviceInfo};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn sample_app() -> App {
        config::init_for_tests();
        let mut app = App::new();
        app.handle_bt_event(BtEvent::Ready);
        app.handle_bt_event(BtEvent::AdapterState(AdapterInfo {
            name: "hci0".into(),
            alias: "laptop".into(),
            powered: true,
            ..AdapterInfo::default()
        }));
        for (n, name, connected, rssi, battery) in [
            (
                1,
                Some("Sony WH-1000XM5 Wireless Noise Cancelling"),
                true,
                Some(-45),
                Some(80),
            ),
            (2, Some("Keychron K2"), false, None, None),
            (3, None, false, Some(-88), None),
        ] {
            let mut d = DeviceInfo::new(Address::new([0, 0, 0, 0, 0, n]));
            d.name = name.map(Into::into);
            d.alias = name.map_or_else(|| d.address.to_string().replace(':', "-"), Into::into);
            d.icon = Some("audio-headset".into());
            d.paired = n < 3;
            d.connected = connected;
            d.rssi = rssi;
            d.battery = battery;
            app.handle_bt_event(BtEvent::DeviceUpdated(d));
        }
        app
    }

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render(f, app)).unwrap();
        let buf = term.backend().buffer();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn renders_at_every_size_without_panicking() {
        let mut app = sample_app();
        for w in [1, 20, 44, 60, 80, 100, 160, 300] {
            for h in [1, 5, 12, 20, 30, 60] {
                draw(&mut app, w, h);
            }
        }
        app.modal = Some(Modal::Help);
        for (w, h) in [(44, 12), (80, 24), (200, 60)] {
            draw(&mut app, w, h);
        }
    }

    #[test]
    fn unnamed_devices_get_a_friendly_label() {
        let mut app = sample_app();
        let screen = draw(&mut app, 120, 30);
        assert!(screen.contains("Unnamed device · 00:03"), "{screen}");
        assert!(screen.contains("MY DEVICES"));
    }

    /// `cargo test dump_screens -- --ignored --nocapture` prints layouts.
    #[test]
    #[ignore = "visual inspection helper"]
    fn dump_screens() {
        let mut app = sample_app();
        for (w, h) in [(60, 16), (90, 30), (130, 30)] {
            println!("──── {w}×{h}\n{}", draw(&mut app, w, h));
        }
        crate::theme::use_plain_glyphs(true);
        println!("──── plain 130×30\n{}", draw(&mut app, 130, 30));
        app.modal = Some(Modal::Help);
        println!("──── help 130×36\n{}", draw(&mut app, 130, 36));
        println!("──── help 80×24\n{}", draw(&mut app, 80, 24));
    }

    #[test]
    fn truncates_by_display_width() {
        assert_eq!(truncate("Headphones", 20), "Headphones");
        assert_eq!(truncate("Headphones", 5), "Head…");
        assert_eq!(truncate("日本語のデバイス", 7), "日本語…");
        assert_eq!(pad("ab", 4), "ab  ");
    }
}
