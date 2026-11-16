//! "Cosmic Dawn" palette, glyphs, and small animation helpers.
//!
//! All colours come from the user's config. No style sets a background, so
//! translucent / blurred terminals keep showing through.
//!
//! Glyphs come in two sets: Nerd Font icons (default) and a plain-Unicode
//! fallback (`general.nerd_font = false` or `--no-nerd-font`) that renders in
//! any font.

use std::sync::atomic::{AtomicBool, Ordering};

use ratatui::style::{Color, Modifier, Style};

use crate::config;

// ─── Palette ────────────────────────────────────────────────────────────────

#[inline]
fn palette() -> &'static config::Palette {
    &config::get().theme.palette
}

pub fn accent() -> Color {
    palette().accent_primary
}
pub fn accent2() -> Color {
    palette().accent_secondary
}
pub fn red() -> Color {
    palette().accent_error
}
pub fn text() -> Color {
    palette().text_primary
}
pub fn muted() -> Color {
    palette().text_dim
}
pub fn amber() -> Color {
    palette().paired
}
pub fn green() -> Color {
    palette().success
}
pub fn pulse() -> Color {
    palette().scanning
}
pub fn edge() -> Color {
    palette().border_inactive
}

// ─── Styles ─────────────────────────────────────────────────────────────────

pub fn fg(color: Color) -> Style {
    Style::default().fg(color)
}
pub fn bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}
pub fn title() -> Style {
    bold(accent())
}
pub fn body() -> Style {
    fg(text())
}
pub fn dim() -> Style {
    fg(muted())
}
pub fn key() -> Style {
    bold(accent())
}
pub fn section() -> Style {
    bold(muted())
}
pub fn border() -> Style {
    fg(edge())
}
pub fn border_focus() -> Style {
    fg(accent())
}
pub fn selected() -> Style {
    bold(accent2())
}
pub fn error() -> Style {
    bold(red())
}

// ─── Glyphs ─────────────────────────────────────────────────────────────────

static PLAIN: AtomicBool = AtomicBool::new(false);

/// Switch to the plain-Unicode glyph set.
pub fn use_plain_glyphs(plain: bool) {
    PLAIN.store(plain, Ordering::Relaxed);
}

fn nerd() -> bool {
    !PLAIN.load(Ordering::Relaxed)
}

macro_rules! glyphs {
    ($($name:ident: $nerd:literal, $plain:literal;)*) => {
        $(pub fn $name() -> &'static str { if nerd() { $nerd } else { $plain } })*
    };
}

glyphs! {
    logo: "󰂯", "ᛒ";
    power: "⏻", "⏻";
    connected_badge: "󰂱", "●";
    paired_badge: "󰌾", "◆";
    trusted_badge: "󰄬", "✓";
    search_icon: "", "/";
    help_icon: "󰋖", "?";
    ok_icon: "", "✓";
    err_icon: "", "✗";
    info_icon: "", "i";
    lock_icon: "󰌾", "◆";
    sort_glyph: "󰒺", "⇅";
    battery_full: "󰁹", "▮";
    battery_mid: "󰁿", "▮";
    battery_low: "󰁻", "▯";
    battery_empty: "󰂎", "▯";
}

/// Map a BlueZ `icon` property (freedesktop icon name) or class to a glyph.
pub fn device_icon(icon: Option<&str>, class: Option<u32>) -> &'static str {
    let kind = device_kind(icon, class);
    if nerd() {
        match kind {
            DeviceKind::Headphones => "󰋋",
            DeviceKind::Speaker => "󰓃",
            DeviceKind::Phone => "󰏲",
            DeviceKind::Computer => "󰌢",
            DeviceKind::Keyboard => "󰌌",
            DeviceKind::Mouse => "󰍽",
            DeviceKind::Gamepad => "󰊗",
            DeviceKind::Tablet => "󰓶",
            DeviceKind::Camera => "󰄀",
            DeviceKind::Printer => "󰐪",
            DeviceKind::Watch => "󰖉",
            DeviceKind::Display => "󰍹",
            DeviceKind::Network => "󰈀",
            DeviceKind::Health => "󰋑",
            DeviceKind::Other => "󰂯",
        }
    } else {
        match kind {
            DeviceKind::Headphones | DeviceKind::Speaker => "♪",
            DeviceKind::Phone => "☎",
            DeviceKind::Keyboard => "⌨",
            DeviceKind::Watch => "◷",
            DeviceKind::Gamepad => "✚",
            DeviceKind::Health => "♥",
            _ => "•",
        }
    }
}

/// Coarse device category, for icons and the "Type" detail row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Headphones,
    Speaker,
    Phone,
    Computer,
    Keyboard,
    Mouse,
    Gamepad,
    Tablet,
    Camera,
    Printer,
    Watch,
    Display,
    Network,
    Health,
    Other,
}

impl DeviceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Headphones => "Headphones",
            Self::Speaker => "Speaker",
            Self::Phone => "Phone",
            Self::Computer => "Computer",
            Self::Keyboard => "Keyboard",
            Self::Mouse => "Mouse",
            Self::Gamepad => "Game controller",
            Self::Tablet => "Tablet",
            Self::Camera => "Camera",
            Self::Printer => "Printer",
            Self::Watch => "Watch",
            Self::Display => "Display",
            Self::Network => "Network",
            Self::Health => "Health device",
            Self::Other => "Bluetooth device",
        }
    }
}

pub fn device_kind(icon: Option<&str>, class: Option<u32>) -> DeviceKind {
    if let Some(icon) = icon {
        let kind = match icon {
            s if s.contains("headset") || s.contains("headphones") => DeviceKind::Headphones,
            s if s.contains("audio") || s.contains("speaker") => DeviceKind::Speaker,
            s if s.contains("phone") => DeviceKind::Phone,
            s if s.contains("computer") => DeviceKind::Computer,
            s if s.contains("keyboard") => DeviceKind::Keyboard,
            s if s.contains("mouse") => DeviceKind::Mouse,
            s if s.contains("gaming") => DeviceKind::Gamepad,
            s if s.contains("tablet") => DeviceKind::Tablet,
            s if s.contains("camera") => DeviceKind::Camera,
            s if s.contains("printer") => DeviceKind::Printer,
            s if s.contains("watch") => DeviceKind::Watch,
            s if s.contains("display") || s.contains("monitor") => DeviceKind::Display,
            s if s.contains("network") || s.contains("modem") => DeviceKind::Network,
            _ => DeviceKind::Other,
        };
        if kind != DeviceKind::Other {
            return kind;
        }
    }
    match class.map(|c| (c >> 8) & 0x1F) {
        Some(1) => DeviceKind::Computer,
        Some(2) => DeviceKind::Phone,
        Some(3) => DeviceKind::Network,
        Some(4) => DeviceKind::Headphones,
        Some(5) => DeviceKind::Keyboard,
        Some(6) => DeviceKind::Printer,
        Some(7) => DeviceKind::Watch,
        Some(9) => DeviceKind::Health,
        _ => DeviceKind::Other,
    }
}

// ─── Signal & battery ───────────────────────────────────────────────────────

/// Signal quality bucket 0–4 for an RSSI reading.
pub fn signal_level(rssi: Option<i16>) -> u8 {
    match rssi {
        Some(r) if r >= -55 => 4,
        Some(r) if r >= -67 => 3,
        Some(r) if r >= -78 => 2,
        Some(_) => 1,
        None => 0,
    }
}

pub fn signal_color(level: u8) -> Color {
    match level {
        4 => green(),
        3 => accent(),
        2 => amber(),
        1 => red(),
        _ => muted(),
    }
}

pub fn signal_label(level: u8) -> &'static str {
    match level {
        4 => "Excellent",
        3 => "Good",
        2 => "Fair",
        1 => "Weak",
        _ => "Out of range",
    }
}

/// Four rising bars: ▂▄▆█, with the unlit ones returned separately.
pub fn signal_bars(level: u8) -> (&'static str, &'static str) {
    const BARS: &str = "▂▄▆█";
    // Each bar is 3 bytes in UTF-8.
    let lit = usize::from(level.min(4)) * 3;
    (&BARS[..lit], &BARS[lit..])
}

pub fn battery_glyph(pct: u8) -> (&'static str, Color) {
    match pct {
        70..=100 => (battery_full(), green()),
        40..=69 => (battery_mid(), accent()),
        15..=39 => (battery_low(), amber()),
        _ => (battery_empty(), red()),
    }
}

// ─── Animation ──────────────────────────────────────────────────────────────

const SPINNER: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Spinner frame for a time in milliseconds (12.5 fps regardless of tick rate).
pub fn spinner(elapsed_ms: u128) -> &'static str {
    let len = SPINNER.len() as u128;
    let idx = usize::try_from((elapsed_ms / 80) % len).unwrap_or(0);
    SPINNER[idx]
}

/// Cubic ease-out: fast start, smooth deceleration. `t` in [0, 1].
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_bars_split_lit_and_unlit() {
        assert_eq!(signal_bars(0), ("", "▂▄▆█"));
        assert_eq!(signal_bars(2), ("▂▄", "▆█"));
        assert_eq!(signal_bars(9), ("▂▄▆█", ""));
    }

    #[test]
    fn kind_falls_back_to_class() {
        assert_eq!(
            device_kind(Some("audio-headset"), None),
            DeviceKind::Headphones
        );
        assert_eq!(device_kind(None, Some(0x240404)), DeviceKind::Headphones);
        assert_eq!(
            device_kind(Some("weird"), Some(0x000540)),
            DeviceKind::Keyboard
        );
        assert_eq!(device_kind(None, None), DeviceKind::Other);
    }
}
