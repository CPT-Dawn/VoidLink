//! Help overlay, generated from the active keybindings.

use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use ratatui::layout::{Constraint, Layout};

use super::{modal_frame, truncate};
use crate::config::{self, key_label};
use crate::theme;

pub fn render(frame: &mut Frame) {
    let kb = &config::get().keys;
    let k = |code| key_label(code);
    let sections: [(&str, Vec<(String, &str)>); 4] = [
        (
            "Navigate",
            vec![
                (format!("{} {} ↑ ↓", k(kb.nav_down), k(kb.nav_up)), "Move"),
                (
                    format!("{} {}", k(kb.jump_top), k(kb.jump_bottom)),
                    "Top / bottom",
                ),
                (k(kb.search), "Search (/ for regex)"),
                (k(kb.cycle_sort), "Sort order"),
                ("Esc".into(), "Clear search"),
                ("Mouse".into(), "Click, double-click"),
            ],
        ),
        (
            "Devices",
            vec![
                (k(kb.connect_toggle), "Connect / disconnect"),
                (k(kb.disconnect), "Disconnect"),
                (k(kb.pair), "Pair only"),
                (k(kb.trust), "Trust / untrust"),
                (k(kb.rename), "Rename"),
                (k(kb.remove), "Forget"),
                (k(kb.refresh), "Refresh"),
            ],
        ),
        (
            "Bluetooth",
            vec![
                (k(kb.toggle_scan), "Scan on / off"),
                (k(kb.toggle_adapter), "Power on / off"),
            ],
        ),
        (
            "App",
            vec![(k(kb.help), "This help"), (k(kb.quit), "Quit (Ctrl+C)")],
        ),
    ];

    let section_lines = |secs: &[(&str, Vec<(String, &str)>)], desc_w: usize| {
        let mut lines = Vec::new();
        for (i, (title, rows)) in secs.iter().enumerate() {
            if i > 0 {
                lines.push(Line::raw(""));
            }
            lines.push(Line::from(Span::styled(
                title.to_uppercase(),
                theme::section(),
            )));
            for (key, desc) in rows {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {key:<10}"), theme::key()),
                    Span::styled(truncate(desc, desc_w), theme::body()),
                ]));
            }
        }
        lines
    };
    let footer = Line::from(Span::styled(
        format!("Config: {}", config::config_path().display()),
        theme::dim(),
    ));
    let title = format!(" {} Keybindings ", theme::help_icon());
    let screen = frame.area();

    let mut single = section_lines(&sections, 44);
    single.push(Line::raw(""));
    single.push(footer.clone());
    let tall = u16::try_from(single.len()).unwrap_or(u16::MAX) + 3;

    if tall <= screen.height || screen.width < 70 {
        let inner = modal_frame(frame, &title, 62, tall, theme::accent2());
        frame.render_widget(Paragraph::new(single), inner);
        return;
    }

    // Short terminal: two columns.
    let width = screen.width.saturating_sub(4).min(110);
    let col_w = usize::from(width.saturating_sub(8) / 2);
    let desc_w = col_w.saturating_sub(13);
    let [nav, devices, bluetooth, app] = sections;
    let left = section_lines(&[nav, app], desc_w);
    let right = section_lines(&[devices, bluetooth], desc_w);
    let rows = left.len().max(right.len());
    let height = u16::try_from(rows).unwrap_or(u16::MAX) + 5;
    let inner = modal_frame(frame, &title, width, height, theme::accent2());
    let [cols, _, foot] = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(inner);
    let [l, _, r] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(2),
        Constraint::Fill(1),
    ])
    .areas(cols);
    frame.render_widget(Paragraph::new(left), l);
    frame.render_widget(Paragraph::new(right), r);
    frame.render_widget(Paragraph::new(footer), foot);
}
