//! Details of the selected device, with the actions available for it.

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::{panel, truncate};
use crate::app::App;
use crate::bluetooth::types::DeviceInfo;
use crate::config::{self, key_label};
use crate::theme;

const LABEL_W: usize = 10;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel(" Details ", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(d) = app.selected_device() else {
        let hint = Paragraph::new(Line::from(Span::styled(
            "Select a device to see its details",
            theme::dim(),
        )))
        .centered();
        let y = inner.y + inner.height / 2;
        frame.render_widget(
            hint,
            Rect {
                y,
                height: 1.min(inner.height),
                ..inner
            },
        );
        return;
    };

    let width = usize::from(inner.width);
    let kind = theme::device_kind(d.icon.as_deref(), d.class);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{}  ", theme::device_icon(d.icon.as_deref(), d.class)),
                theme::bold(theme::accent()),
            ),
            Span::styled(
                truncate(&d.label(), width.saturating_sub(3)),
                theme::bold(theme::text()),
            ),
        ]),
        Line::from(Span::styled(
            format!("   {} · {}", kind.label(), d.address),
            theme::dim(),
        )),
        Line::raw(""),
    ];

    lines.push(row("Status", status_spans(app, d)));
    lines.push(row(
        "Paired",
        vec![
            yes_no(d.paired, theme::amber()),
            Span::styled("   Trusted ", theme::dim()),
            yes_no(d.trusted, theme::green()),
        ],
    ));

    let level = theme::signal_level(d.rssi);
    let (lit, unlit) = theme::signal_bars(level);
    let mut signal = vec![
        Span::styled(lit, theme::fg(theme::signal_color(level))),
        Span::styled(unlit, theme::fg(theme::edge())),
        Span::styled(format!("  {}", theme::signal_label(level)), theme::body()),
    ];
    if let Some(rssi) = d.rssi {
        signal.push(Span::styled(format!(" ({rssi} dBm)"), theme::dim()));
    }
    lines.push(row("Signal", signal));

    if let Some(pct) = d.battery {
        let (_, color) = theme::battery_glyph(pct);
        let bar_w = 10usize;
        let lit = usize::from(pct).div_ceil(10).min(bar_w);
        lines.push(row(
            "Battery",
            vec![
                Span::styled("━".repeat(lit), theme::fg(color)),
                Span::styled("━".repeat(bar_w - lit), theme::fg(theme::edge())),
                Span::styled(format!("  {pct}%"), theme::body()),
            ],
        ));
    }

    // Actions for this device.
    if inner.height as usize > lines.len() + 3 {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("ACTIONS ", theme::section()),
            Span::styled(
                "─".repeat(width.saturating_sub(8)),
                theme::fg(theme::edge()),
            ),
        ]));
        for (key, label) in actions(d) {
            lines.push(Line::from(vec![
                Span::styled(format!("{key:<LABEL_W$}"), theme::key()),
                Span::styled(label, theme::body()),
            ]));
        }
    }

    frame.render_widget(Paragraph::new(lines), inner);
}

fn row(label: &str, mut value: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = vec![Span::styled(format!("{label:<LABEL_W$}"), theme::dim())];
    spans.append(&mut value);
    Line::from(spans)
}

fn yes_no(v: bool, color: ratatui::style::Color) -> Span<'static> {
    if v {
        Span::styled("Yes", theme::bold(color))
    } else {
        Span::styled("No", theme::dim())
    }
}

fn status_spans(app: &App, d: &DeviceInfo) -> Vec<Span<'static>> {
    if let Some(op) = app.pending.get(&d.address) {
        return vec![Span::styled(
            format!(
                "{} {}…",
                theme::spinner(app.elapsed_ms()),
                op.progress_label()
            ),
            theme::bold(theme::accent()),
        )];
    }
    if d.connected {
        vec![Span::styled("● Connected", theme::bold(theme::accent()))]
    } else if d.rssi.is_some() {
        vec![Span::styled("○ Not connected", theme::body())]
    } else {
        vec![Span::styled("○ Not connected · out of range", theme::dim())]
    }
}

fn actions(d: &DeviceInfo) -> Vec<(String, &'static str)> {
    let kb = &config::get().keys;
    let mut out = vec![(
        key_label(kb.connect_toggle),
        if d.connected {
            "Disconnect"
        } else if d.paired {
            "Connect"
        } else {
            "Pair & connect"
        },
    )];
    if !d.paired {
        out.push((key_label(kb.pair), "Pair only"));
    }
    out.push((
        key_label(kb.trust),
        if d.trusted {
            "Untrust"
        } else {
            "Trust (auto-reconnect)"
        },
    ));
    out.push((key_label(kb.rename), "Rename"));
    if d.is_known() {
        out.push((key_label(kb.remove), "Forget device"));
    }
    out
}
