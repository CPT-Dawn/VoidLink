//! The device list: grouped rows with status, battery and signal columns,
//! plus friendly empty states.

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use super::{pad, panel, truncate};
use crate::app::{App, BtStatus, Row};
use crate::bluetooth::types::DeviceInfo;
use crate::config::{self, key_label};
use crate::theme;

/// Fixed column widths (display cells).
const STATUS_W: usize = 15;
const BATTERY_W: usize = 6;
const SIGNAL_W: usize = 4;

pub fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let mut block = panel(" Devices ", true);
    if app.scanning {
        block = block.title_top(
            Line::from(Span::styled(
                format!(" {} Scanning ", theme::spinner(app.elapsed_ms())),
                theme::bold(theme::pulse()),
            ))
            .right_aligned(),
        );
    } else if app.filter_active() {
        block = block.title_top(
            Line::from(Span::styled(
                format!(" {} {} ", theme::search_icon(), app.search_query),
                theme::bold(theme::accent2()),
            ))
            .right_aligned(),
        );
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);
    app.list_area = inner;

    if app.rows.is_empty() {
        render_empty(frame, app, inner);
        return;
    }

    // 2 cells are taken by the selection marker.
    let width = usize::from(inner.width).saturating_sub(2);
    let show_battery = width >= 40;
    let show_status = width >= 30;
    let fixed = 3
        + SIGNAL_W
        + 1
        + if show_status { STATUS_W + 1 } else { 0 }
        + if show_battery { BATTERY_W + 1 } else { 0 };
    let name_w = width.saturating_sub(fixed).max(6);

    let items: Vec<ListItem> = app
        .rows
        .iter()
        .map(|row| match *row {
            Row::Header(section, count) => {
                let label = format!("{} · {count} ", section.title());
                let rule = "─".repeat(width.saturating_sub(label.chars().count()));
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(label, theme::section()),
                    Span::styled(rule, theme::fg(theme::edge())),
                ]))
            }
            Row::Device(i) => {
                let d = &app.devices[i];
                device_row(app, d, name_w, show_status, show_battery)
            }
        })
        .collect();

    let list = List::new(items);
    app.list_state.select(app.selected_row());
    frame.render_stateful_widget(list, inner, &mut app.list_state);
}

fn device_row<'a>(
    app: &App,
    d: &DeviceInfo,
    name_w: usize,
    show_status: bool,
    show_battery: bool,
) -> ListItem<'a> {
    let selected = app.selected == Some(d.address);
    let pending = app.pending.get(&d.address).copied();

    let icon_style = if d.connected {
        theme::bold(theme::accent())
    } else {
        theme::dim()
    };
    let name_style = if selected {
        theme::selected()
    } else if d.connected {
        theme::bold(theme::text())
    } else if (d.rssi.is_none() && !d.is_known()) || d.is_unnamed() {
        theme::dim()
    } else {
        theme::body()
    };

    let marker = if selected {
        Span::styled("▌ ", theme::bold(theme::accent2()))
    } else {
        Span::raw("  ")
    };
    let mut spans = vec![
        marker,
        Span::styled(
            format!("{} ", theme::device_icon(d.icon.as_deref(), d.class)),
            icon_style,
        ),
        Span::styled(pad(&truncate(&d.label(), name_w), name_w), name_style),
        Span::raw(" "),
    ];

    if show_status {
        let (text, style) = match pending {
            Some(op) => (
                format!(
                    "{} {}…",
                    theme::spinner(app.elapsed_ms()),
                    op.progress_label()
                ),
                theme::bold(theme::accent()),
            ),
            None if d.connected => (
                format!("{} Connected", theme::connected_badge()),
                theme::bold(theme::accent()),
            ),
            None if d.paired => (
                format!("{} Paired", theme::paired_badge()),
                theme::fg(theme::amber()),
            ),
            None if d.trusted => (
                format!("{} Trusted", theme::trusted_badge()),
                theme::fg(theme::green()),
            ),
            None => (String::new(), theme::dim()),
        };
        spans.push(Span::styled(
            pad(&truncate(&text, STATUS_W), STATUS_W),
            style,
        ));
        spans.push(Span::raw(" "));
    }

    if show_battery {
        match d.battery {
            Some(pct) => {
                let (glyph, color) = theme::battery_glyph(pct);
                spans.push(Span::styled(
                    pad(&format!("{glyph}{pct:>3}%"), BATTERY_W),
                    theme::fg(color),
                ));
            }
            None => spans.push(Span::raw(" ".repeat(BATTERY_W))),
        }
        spans.push(Span::raw(" "));
    }

    let level = theme::signal_level(d.rssi);
    let (lit, unlit) = theme::signal_bars(level);
    spans.push(Span::styled(
        lit,
        Style::default().fg(theme::signal_color(level)),
    ));
    spans.push(Span::styled(unlit, theme::fg(theme::edge())));

    ListItem::new(Line::from(spans))
}

fn render_empty(frame: &mut Frame, app: &App, area: Rect) {
    let kb = &config::get().keys;
    let key = |k| Span::styled(key_label(k), theme::key());

    let lines: Vec<Line> = match &app.status {
        BtStatus::Starting => vec![Line::from(Span::styled(
            format!(
                "{} Connecting to Bluetooth…",
                theme::spinner(app.elapsed_ms())
            ),
            theme::dim(),
        ))],
        BtStatus::Unavailable(_) => vec![],
        BtStatus::Ready if !app.adapter.powered => vec![
            Line::from(Span::styled(
                format!("{} Bluetooth is off", theme::power()),
                theme::title(),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Press ", theme::dim()),
                key(kb.toggle_adapter),
                Span::styled(" to turn it on", theme::dim()),
            ]),
        ],
        BtStatus::Ready if app.filter_active() => vec![
            Line::from(Span::styled(
                format!("No devices match “{}”", app.search_query),
                theme::body(),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Press ", theme::dim()),
                Span::styled("Esc", theme::key()),
                Span::styled(" to clear the filter", theme::dim()),
            ]),
        ],
        BtStatus::Ready if app.scanning => vec![
            Line::from(Span::styled(
                format!("{} Looking for devices…", theme::spinner(app.elapsed_ms())),
                theme::bold(theme::pulse()),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                "Put your device in pairing mode and keep it nearby",
                theme::dim(),
            )),
        ],
        BtStatus::Ready => vec![
            Line::from(Span::styled(
                format!("{}  No devices yet", theme::logo()),
                theme::title(),
            )),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Press ", theme::dim()),
                key(kb.toggle_scan),
                Span::styled(" to scan for nearby devices", theme::dim()),
            ]),
            Line::from(vec![
                Span::styled("Press ", theme::dim()),
                key(kb.help),
                Span::styled(" for all keys", theme::dim()),
            ]),
        ],
    };

    let height = u16::try_from(lines.len()).unwrap_or(0);
    let y = area.y + area.height.saturating_sub(height) / 2;
    let rect = Rect {
        y,
        height: height.min(area.height),
        ..area
    };
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), rect);
}
