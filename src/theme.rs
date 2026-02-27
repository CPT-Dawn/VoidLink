//! "Void Aurora" palette and Nerd Font icon mappings.
//!
//! All colors are resolved from the user's config at startup.  Style
//! functions read from the global `config::get()` singleton — the only
//! overhead is a pointer dereference per call.
//!
//! Design principles:
//! - **No hardcoded backgrounds.**  Every style uses `Color::Reset` (or omits
//!   `.bg()`) so the terminal's native background — including compositor blur —
//!   shines through.

use ratatui::style::{Color, Modifier, Style};

use crate::config;

// ─── Config-backed palette accessors ────────────────────────────────────────

#[inline]
fn palette() -> &'static config::Palette {
    &config::get().theme.palette
}

/// Primary accent — arctic sky.
pub fn cyan() -> Color {
    palette().accent_primary
}

/// Secondary accent — soft violet.
pub fn deep_purple() -> Color {
    palette().accent_secondary
}

/// Error accent — dusty rose.
pub fn dawn_red() -> Color {
    palette().accent_error
}

/// Primary body text — pearl white.
pub fn text_primary() -> Color {
    palette().text_primary
}

/// Dimmed / secondary text — cool slate.
pub fn text_dim() -> Color {
    palette().text_dim
}

/// Paired-but-not-connected — warm honey.
pub fn amber() -> Color {
    palette().paired
}

/// Success / trusted — fresh mint.
pub fn green() -> Color {
    palette().success
}

/// Scanning spinner pulse — ice glow.
pub fn scanning_pulse() -> Color {
    palette().scanning
}

// ─── Composite styles ───────────────────────────────────────────────────────

/// Title / header style.
pub fn title() -> Style {
    Style::default().fg(cyan()).add_modifier(Modifier::BOLD)
}

/// Normal list item.
pub fn list_item() -> Style {
    Style::default().fg(text_primary())
}

/// Dimmed / secondary label.
pub fn dim() -> Style {
    Style::default().fg(text_dim())
}

/// Currently selected row highlight — soft violet reversed bar.
pub fn selected() -> Style {
    Style::default()
        .fg(deep_purple())
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}

/// Connected device accent.
pub fn connected() -> Style {
    Style::default().fg(cyan()).add_modifier(Modifier::BOLD)
}

/// Paired-but-not-connected.
pub fn paired() -> Style {
    Style::default().fg(amber())
}

/// Error / disconnect.
pub fn error() -> Style {
    Style::default().fg(dawn_red()).add_modifier(Modifier::BOLD)
}

/// Trusted badge.
pub fn trusted() -> Style {
    Style::default().fg(green())
}

/// Active border (focused pane).
pub fn border_active() -> Style {
    Style::default().fg(cyan())
}

/// Inactive border — slightly lighter than dim text so panes stay legible.
pub fn border_inactive() -> Style {
    Style::default().fg(palette().border_inactive)
}

// ─── RSSI signal strength helpers ───────────────────────────────────────────

/// Return a Nerd Font signal-strength icon and colour for the given RSSI.
pub fn rssi_display(rssi: Option<i16>) -> (&'static str, Color) {
    match rssi {
        Some(r) if r >= -50 => ("󰤨", green()),    // excellent
        Some(r) if r >= -60 => ("󰤥", cyan()),     // good
        Some(r) if r >= -70 => ("󰤢", amber()),    // fair
        Some(r) if r >= -80 => ("󰤟", dawn_red()), // weak
        Some(_) => ("󰤯", dawn_red()),             // very weak
        None => ("󰤮", text_dim()),                // unknown / not in range
    }
}

/// Return an RSSI bar string (1-5 blocks).
pub fn rssi_bar(rssi: Option<i16>) -> &'static str {
    match rssi {
        Some(r) if r >= -50 => "█████",
        Some(r) if r >= -60 => "████░",
        Some(r) if r >= -70 => "███░░",
        Some(r) if r >= -80 => "██░░░",
        Some(_) => "█░░░░",
        None => "░░░░░",
    }
}

// ─── Device type → Nerd Font icon mapping ───────────────────────────────────

/// Map a BlueZ `icon` property (freedesktop icon name) to a Nerd Font glyph.
pub fn device_icon(icon: Option<&str>, class: Option<u32>) -> &'static str {
    if let Some(icon) = icon {
        match icon {
            s if s.contains("audio-headset") || s.contains("audio-headphones") => "\u{f025}",
            s if s.contains("audio-card") || s.contains("speaker") => "󰓃",
            s if s.contains("phone") => "\u{f095}",
            s if s.contains("computer") => "󰍽",
            s if s.contains("input-keyboard") => "󰌌",
            s if s.contains("input-mouse") => "󰍽",
            s if s.contains("input-gaming") => "󰊗",
            s if s.contains("input-tablet") => "󰓶",
            s if s.contains("camera") => "󰄀",
            s if s.contains("printer") => "󰐪",
            s if s.contains("network") => "󰈀",
            s if s.contains("video-display") || s.contains("monitor") => "󰍹",
            _ => "󰂯",
        }
    } else if let Some(cls) = class {
        let major = (cls >> 8) & 0x1F;
        match major {
            1 => "󰍽",
            2 => "\u{f095}",
            3 => "󰈀",
            4 => "󰓃",
            5 => "󰌌",
            6 => "󰄀",
            7 => "󰌚",
            _ => "󰂯",
        }
    } else {
        "󰂯"
    }
}

// ─── Spinner frames ─────────────────────────────────────────────────────────

/// Braille-dot spinner frames for the scanning animation.
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Get the current spinner frame for a given tick count.
pub fn spinner_frame(tick: u64) -> &'static str {
    SPINNER_FRAMES[(tick as usize) % SPINNER_FRAMES.len()]
}

// ─── Battery display ────────────────────────────────────────────────────────

/// Return a Nerd Font battery icon and colour for the given percentage.
pub fn battery_display(pct: Option<u8>) -> (&'static str, Color) {
    match pct {
        Some(p) if p >= 80 => ("󰁹", green()),
        Some(p) if p >= 60 => ("󰂁", cyan()),
        Some(p) if p >= 40 => ("󰁿", amber()),
        Some(p) if p >= 20 => ("󰁻", dawn_red()),
        Some(_) => ("󰁺", dawn_red()),
        None => ("󰂃", text_dim()),
    }
}
