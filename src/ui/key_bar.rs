//! Bottom hint bar. Shows the keys that matter right now (using the user's
//! remapped keys), dropping lower-priority hints on narrow terminals. In
//! search and rename modes it becomes the input line.

use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::hint;
use crate::app::{App, Modal, Mode};
use crate::config::{self, key_label};
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let line = match (&app.modal, app.mode) {
        (Some(_), _) => modal_hints(app),
        (None, Mode::Search) => input_line(
            &format!("{} ", theme::search_icon()),
            &app.search_query,
            app.search_error.as_deref(),
            "Enter keep · Esc clear · start with / for regex",
        ),
        (None, Mode::Rename) => input_line(
            "Rename  ",
            &app.rename_buffer,
            None,
            "Enter save · Esc cancel",
        ),
        (None, Mode::Normal) => fit(normal_hints(app), usize::from(area.width)),
    };
    let alignment = if app.mode == Mode::Normal || app.modal.is_some() {
        Alignment::Center
    } else {
        Alignment::Left
    };
    frame.render_widget(Paragraph::new(line).alignment(alignment), area);
}

fn input_line(prompt: &str, text: &str, error: Option<&str>, help: &str) -> Line<'static> {
    let mut spans = vec![
        Span::styled(prompt.to_string(), theme::bold(theme::accent2())),
        Span::styled(text.to_string(), theme::bold(theme::text())),
        Span::styled("▏", theme::bold(theme::accent())),
        Span::raw("   "),
    ];
    match error {
        Some(e) => spans.push(Span::styled(e.to_string(), theme::error())),
        None => spans.push(Span::styled(help.to_string(), theme::dim())),
    }
    Line::from(spans)
}

/// Hints in priority order; the last two (help, quit) are always kept.
fn normal_hints(app: &App) -> Vec<Vec<Span<'static>>> {
    let kb = &config::get().keys;
    let k = |code| key_label(code);
    let mut hints = Vec::new();

    if let Some(d) = app.selected_device() {
        let label = if d.connected { "Disconnect" } else { "Connect" };
        hints.push(hint(&k(kb.connect_toggle), label));
    }
    hints.push(hint(
        &k(kb.toggle_scan),
        if app.scanning { "Stop scan" } else { "Scan" },
    ));
    hints.push(hint(&k(kb.search), "Search"));
    if let Some(d) = app.selected_device() {
        if !d.paired {
            hints.push(hint(&k(kb.pair), "Pair"));
        }
        hints.push(hint(
            &k(kb.trust),
            if d.trusted { "Untrust" } else { "Trust" },
        ));
        hints.push(hint(&k(kb.rename), "Rename"));
        if d.is_known() {
            hints.push(hint(&k(kb.remove), "Forget"));
        }
    }
    hints.push(hint(
        &k(kb.toggle_adapter),
        if app.adapter.powered {
            "Power off"
        } else {
            "Power on"
        },
    ));
    hints.push(hint(&k(kb.cycle_sort), "Sort"));
    hints.push(hint(&k(kb.help), "Help"));
    hints.push(hint(&k(kb.quit), "Quit"));
    hints
}

fn modal_hints(app: &App) -> Line<'static> {
    let hints = match &app.modal {
        Some(Modal::Prompt(p)) if p.needs_input() => {
            vec![hint("Enter", "Submit"), hint("Esc", "Cancel")]
        }
        Some(Modal::Prompt(_)) => vec![hint("y", "Accept"), hint("n", "Reject")],
        Some(Modal::ConfirmRemove(_)) => vec![hint("y", "Forget"), hint("n", "Cancel")],
        _ => vec![hint("Esc", "Close")],
    };
    fit(hints, usize::MAX)
}

/// Join hints with gaps, keeping as many as fit. The final two are reserved.
fn fit(hints: Vec<Vec<Span<'static>>>, width: usize) -> Line<'static> {
    const GAP: usize = 3;
    let w = |h: &Vec<Span>| h.iter().map(Span::width).sum::<usize>();
    let n = hints.len();
    let reserved: usize = hints
        .iter()
        .skip(n.saturating_sub(2))
        .map(|h| w(h) + GAP)
        .sum();

    let mut used = reserved;
    let mut spans = Vec::new();
    for (i, h) in hints.into_iter().enumerate() {
        let is_reserved = i + 2 >= n;
        if !is_reserved {
            if used + w(&h) + GAP > width {
                continue;
            }
            used += w(&h) + GAP;
        }
        if !spans.is_empty() {
            spans.push(Span::raw(" ".repeat(GAP)));
        }
        spans.extend(h);
    }
    Line::from(spans)
}
