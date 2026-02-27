//! Top-level UI render dispatch.
//!
//! Splits the terminal frame into three regions:
//! - Status bar (top, 3 lines)
//! - Device list (left ~60%) + Detail panel (right ~40%)
//! - Popup overlay (centered, on top of everything)

pub mod detail_panel;
pub mod device_list;
pub mod help;
pub mod key_bar;
pub mod popup;
pub mod spinner;
pub mod status_bar;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;

use crate::app::App;
use crate::config;

/// Render the entire UI.
pub fn render(frame: &mut Frame, app: &App) {
    let list_pct = config::get().general.device_list_percent;
    let detail_pct = 100u16.saturating_sub(list_pct);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // status bar
            Constraint::Min(0),    // main content
            Constraint::Length(1), // key hints bar
        ])
        .split(frame.area());

    // Destructure to avoid raw indexing panics.
    let [status_area, content_area, keybar_area] =
        match <[ratatui::layout::Rect; 3]>::try_from(outer.as_ref()) {
            Ok(a) => a,
            Err(_) => return,
        };

    // ── Status bar ──────────────────────────────────────────────────────
    status_bar::render(frame, app, status_area);

    // ── Main content: device list + detail panel ────────────────────────
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(list_pct),   // device list
            Constraint::Percentage(detail_pct), // detail panel
        ])
        .split(content_area);

    let [list_area, detail_area] = match <[ratatui::layout::Rect; 2]>::try_from(main.as_ref()) {
        Ok(a) => a,
        Err(_) => return,
    };

    device_list::render(frame, app, list_area);
    detail_panel::render(frame, app, detail_area);

    // ── Key hints bar (bottom) ──────────────────────────────────────────
    key_bar::render(frame, app, keybar_area);

    // ── Popup overlay (rendered last so it's on top) ────────────────────
    if let Some(ref popup_data) = app.active_popup {
        popup::render(frame, app, popup_data);
    }
}
