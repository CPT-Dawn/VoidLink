//! Top-level UI render dispatch.
//!
//! Splits the terminal frame into three regions:
//! - Status bar (top, 3 lines)
//! - Device list (left ~60%) + Detail panel (right ~40%)
//! - Popup overlay (centered, on top of everything)

pub mod device_list;
pub mod detail_panel;
pub mod help;
pub mod popup;
pub mod spinner;
pub mod status_bar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::App;

/// Render the entire UI.
pub fn render(frame: &mut Frame, app: &App) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // status bar
            Constraint::Min(0),    // main content
        ])
        .split(frame.area());

    // ── Status bar ──────────────────────────────────────────────────────
    status_bar::render(frame, app, outer[0]);

    // ── Main content: device list + detail panel ────────────────────────
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(55), // device list
            Constraint::Percentage(45), // detail panel
        ])
        .split(outer[1]);

    device_list::render(frame, app, main[0]);
    detail_panel::render(frame, app, main[1]);

    // ── Popup overlay (rendered last so it's on top) ────────────────────
    if let Some(ref popup_data) = app.active_popup {
        popup::render(frame, app, popup_data);
    }
}
