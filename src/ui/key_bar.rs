//! Persistent key instruction bar at the bottom of the screen.
//!
//! Shows context-aware keybindings in a compact, styled row that adapts
//! to the current input mode (Normal, Search, Dialog, Rename).

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, InputMode};
use crate::theme;

/// Render the key-hint bar into the given area.
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let spans = match app.input_mode {
        InputMode::Normal => normal_hints(app),
        InputMode::Search => search_hints(),
        InputMode::Dialog => dialog_hints(),
        InputMode::Rename => rename_hints(),
    };

    let line = Line::from(spans);
    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}

/// Key style: accented, bold, slightly reversed for a modern "badge" look.
fn key(s: &str) -> Span<'_> {
    Span::styled(
        format!(" {s} "),
        Style::default()
            .fg(theme::text_primary())
            .bg(theme::deep_purple())
            .add_modifier(Modifier::BOLD),
    )
}

/// Description style: dimmed.
fn desc(s: &str) -> Span<'_> {
    Span::styled(format!(" {s} "), Style::default().fg(theme::text_dim()))
}

/// Separator between groups.
fn sep() -> Span<'static> {
    Span::raw("   ")
}

fn normal_hints(app: &App) -> Vec<Span<'static>> {
    let mut hints: Vec<Span<'static>> = Vec::with_capacity(40);

    // Navigation.
    hints.push(key("↑/↓"));
    hints.push(desc("Navigate"));
    hints.push(sep());

    // Connect / disconnect (contextual).
    if let Some(device) = app.selected_device() {
        if device.connected {
            hints.push(key("⏎"));
            hints.push(desc("Disconnect"));
        } else {
            hints.push(key("⏎"));
            hints.push(desc("Connect"));
        }
    } else {
        hints.push(key("⏎"));
        hints.push(desc("Connect"));
    }
    hints.push(sep());

    hints.push(key("p"));
    hints.push(desc("Pair"));
    hints.push(sep());

    hints.push(key("t"));
    hints.push(desc("Trust"));
    hints.push(sep());

    hints.push(key("d"));
    hints.push(desc("Disconnect"));
    hints.push(sep());

    hints.push(key("r"));
    hints.push(desc("Remove"));
    hints.push(sep());

    hints.push(key("A"));
    hints.push(desc("Rename"));
    hints.push(sep());

    // Adapter.
    if app.adapter.powered {
        hints.push(key("a"));
        hints.push(desc("Power Off"));
    } else {
        hints.push(key("a"));
        hints.push(desc("Power On"));
    }
    hints.push(sep());

    if app.scanning {
        hints.push(key("s"));
        hints.push(desc("Stop Scan"));
    } else {
        hints.push(key("s"));
        hints.push(desc("Scan"));
    }
    hints.push(sep());

    hints.push(key("/"));
    hints.push(desc("Search"));
    hints.push(sep());

    hints.push(key("?"));
    hints.push(desc("Help"));
    hints.push(sep());

    hints.push(key("q"));
    hints.push(desc("Quit"));

    hints
}

fn search_hints() -> Vec<Span<'static>> {
    vec![
        key("⏎"),
        desc("Confirm"),
        sep(),
        key("Esc"),
        desc("Cancel"),
        sep(),
        desc("Type to filter devices… (prefix / for regex)"),
    ]
}

fn dialog_hints() -> Vec<Span<'static>> {
    vec![key("Esc"), desc("Dismiss"), sep(), key("⏎"), desc("OK")]
}

fn rename_hints() -> Vec<Span<'static>> {
    vec![
        key("⏎"),
        desc("Confirm Rename"),
        sep(),
        key("Esc"),
        desc("Cancel"),
        sep(),
        desc("Type new alias…"),
    ]
}
