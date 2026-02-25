//! "Cosmic Dawn" color palette and Nerd Font icon mappings.
//!
//! Design principles:
//! - **No hardcoded backgrounds.** Every style uses `Color::Reset` (or omits
//!   `.bg()`) so the terminal's native background — including compositor blur —
//!   shines through.
//! - Three accent colours: Cyan, Deep Purple, Dawn Red.
//! - Muted variants for secondary text.

use ratatui::style::{Color, Modifier, Style};

// ─── Cosmic Dawn palette ────────────────────────────────────────────────────

/// Primary accent — headers, active borders, connected indicators.
pub const CYAN: Color = Color::Rgb(0, 212, 255); // #00D4FF

/// Secondary accent — selected items, highlights.
pub const DEEP_PURPLE: Color = Color::Rgb(123, 47, 190); // #7B2FBE

/// Tertiary accent — warnings, disconnect actions, errors.
pub const DAWN_RED: Color = Color::Rgb(255, 107, 107); // #FF6B6B

/// Soft white for primary text.
pub const TEXT_PRIMARY: Color = Color::Rgb(220, 220, 230); // #DCDCE6

/// Dimmed text for secondary labels / inactive items.
pub const TEXT_DIM: Color = Color::Rgb(130, 130, 150); // #828296

/// Paired-but-not-connected indicator.
pub const AMBER: Color = Color::Rgb(255, 183, 77); // #FFB74D

/// Success / trusted indicator.
pub const GREEN: Color = Color::Rgb(105, 240, 174); // #69F0AE

/// Spinner / scanning pulse.
pub const SCANNING_PULSE: Color = Color::Rgb(0, 230, 255); // #00E6FF

// ─── Composite styles ───────────────────────────────────────────────────────

/// Title / header style.
pub fn title() -> Style {
    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
}

/// Normal list item.
pub fn list_item() -> Style {
    Style::default().fg(TEXT_PRIMARY)
}

/// Dimmed / secondary label.
pub fn dim() -> Style {
    Style::default().fg(TEXT_DIM)
}

/// Currently selected row highlight — deep purple foreground, no bg.
pub fn selected() -> Style {
    Style::default()
        .fg(DEEP_PURPLE)
        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
}

/// Connected device accent.
pub fn connected() -> Style {
    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
}

/// Paired-but-not-connected.
pub fn paired() -> Style {
    Style::default().fg(AMBER)
}

/// Error / disconnect.
pub fn error() -> Style {
    Style::default().fg(DAWN_RED).add_modifier(Modifier::BOLD)
}

/// Trusted badge.
pub fn trusted() -> Style {
    Style::default().fg(GREEN)
}

/// Active border (focused pane).
pub fn border_active() -> Style {
    Style::default().fg(CYAN)
}

/// Inactive border.
pub fn border_inactive() -> Style {
    Style::default().fg(TEXT_DIM)
}

// ─── RSSI signal strength helpers ───────────────────────────────────────────

/// Return a Nerd Font signal-strength icon and colour for the given RSSI.
pub fn rssi_display(rssi: Option<i16>) -> (&'static str, Color) {
    match rssi {
        Some(r) if r >= -50 => ("󰤨", GREEN),       // excellent
        Some(r) if r >= -60 => ("󰤥", CYAN),        // good
        Some(r) if r >= -70 => ("󰤢", AMBER),       // fair
        Some(r) if r >= -80 => ("󰤟", DAWN_RED),    // weak
        Some(_) => ("󰤯", DAWN_RED),                 // very weak
        None => ("󰤮", TEXT_DIM),                     // unknown / not in range
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
    // First try the icon string from BlueZ.
    if let Some(icon) = icon {
        match icon {
            s if s.contains("audio-headset") || s.contains("audio-headphones") => "\u{f025}",  // 
            s if s.contains("audio-card") || s.contains("speaker") => "󰓃", // 󰓃
            s if s.contains("phone") => "\u{f095}",                          // 
            s if s.contains("computer") => "󰍽",                              // 󰍽
            s if s.contains("input-keyboard") => "󰌌",                        // 󰌌
            s if s.contains("input-mouse") => "󰍽",                           // 󰍽
            s if s.contains("input-gaming") => "󰊗",                          // 󰊗
            s if s.contains("input-tablet") => "󰓶",                          // 󰓶
            s if s.contains("camera") => "󰄀",                                // 󰄀
            s if s.contains("printer") => "󰐪",                               // 󰐪
            s if s.contains("network") => "󰈀",                               // 󰈀
            s if s.contains("video-display") || s.contains("monitor") => "󰍹", // 󰍹
            _ => "󰂯",                                                         // 󰂯 generic BT
        }
    } else if let Some(cls) = class {
        // Fall back to BT major device class (bits 12..8).
        let major = (cls >> 8) & 0x1F;
        match major {
            1 => "󰍽",  // Computer
            2 => "\u{f095}",  // Phone
            3 => "󰈀",  // LAN / Network Access
            4 => "󰓃",  // Audio/Video
            5 => "󰌌",  // Peripheral (keyboard/mouse/joystick)
            6 => "󰄀",  // Imaging (camera/printer)
            7 => "󰌚",  // Wearable
            _ => "󰂯",  // Unknown
        }
    } else {
        "󰂯" // Default Bluetooth icon
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
        Some(p) if p >= 80 => ("󰁹", GREEN),
        Some(p) if p >= 60 => ("󰂁", CYAN),
        Some(p) if p >= 40 => ("󰁿", AMBER),
        Some(p) if p >= 20 => ("󰁻", DAWN_RED),
        Some(_) => ("󰁺", DAWN_RED),
        None => ("󰂃", TEXT_DIM),
    }
}
