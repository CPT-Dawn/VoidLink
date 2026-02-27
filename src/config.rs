//! Configuration system with embedded defaults and XDG-compliant paths.
//!
//! Boot sequence:
//! 1. Parse the embedded `default_config.toml` (compile-time guarantee it exists).
//! 2. Resolve `~/.config/voidlink/config.toml` via the `directories` crate.
//! 3. If the user file doesn't exist, create the directory tree and write the default.
//! 4. Parse the user file (falling back to embedded defaults on any error).
//! 5. Store the resolved `Config` in a `OnceLock` for zero-cost global access.
//!
//! Every other module calls `config::get()` to obtain a `&'static Config`.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use color_eyre::eyre::{eyre, WrapErr};
use color_eyre::Result;
use crossterm::event::KeyCode;
use ratatui::style::Color;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tracing::{info, warn};

/// Embedded default configuration — baked into the binary at compile time.
const DEFAULT_CONFIG_STR: &str = include_str!("../default_config.toml");

/// Application-wide config singleton.
static CONFIG: OnceLock<Config> = OnceLock::new();

// ─── Public API ─────────────────────────────────────────────────────────────

/// Initialise the configuration system.  Must be called exactly once at
/// startup, **after** tracing and before any other module calls `get()`.
pub fn init() -> Result<()> {
    let config = load()?;
    CONFIG
        .set(config)
        .map_err(|_| eyre!("Config already initialised"))?;
    Ok(())
}

/// Return a static reference to the loaded configuration.
/// # Panics
/// Panics if `init()` has not been called yet.
pub fn get() -> &'static Config {
    CONFIG.get().expect("config::init() was not called")
}

// ─── Loading logic ──────────────────────────────────────────────────────────

fn load() -> Result<Config> {
    // 1. Parse compiled-in defaults — the infallible baseline.
    let defaults: RawConfig = toml::from_str(DEFAULT_CONFIG_STR)
        .wrap_err("BUG: failed to parse embedded default_config.toml")?;

    // 2. Resolve user config path.
    let user_path = config_path();
    info!("Config path: {}", user_path.display());

    // 3. Bootstrap on first run.
    ensure_config_file(&user_path)?;

    // 4. Parse user file; fall back to embedded defaults on *any* error.
    let raw = match fs::read_to_string(&user_path) {
        Ok(contents) => match toml::from_str::<RawConfig>(&contents) {
            Ok(parsed) => {
                info!("Loaded user config from {}", user_path.display());
                parsed
            }
            Err(e) => {
                warn!(
                    "Parse error in {}: {e} — falling back to defaults",
                    user_path.display()
                );
                defaults
            }
        },
        Err(e) => {
            warn!(
                "Cannot read {}: {e} — falling back to defaults",
                user_path.display()
            );
            defaults
        }
    };

    Ok(Config::from(raw))
}

/// Resolve the XDG-compliant config file path.
fn config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "voidlink")
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .unwrap_or_else(|| {
            // Fallback when $HOME is somehow unset (extremely rare).
            PathBuf::from(".config/voidlink/config.toml")
        })
}

/// Create the config directory tree and write the default file if absent.
fn ensure_config_file(path: &PathBuf) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Failed to create config dir: {}", parent.display()))?;
    }
    fs::write(path, DEFAULT_CONFIG_STR)
        .wrap_err_with(|| format!("Failed to write default config to {}", path.display()))?;
    info!("Created default config at {}", path.display());
    Ok(())
}

// ─── Hex colour helper ─────────────────────────────────────────────────────

/// Parse a `#RRGGBB` hex string into an RGB `Color`.
fn parse_hex_color(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

/// Newtype that serialises as `"#RRGGBB"` and deserialises from the same.
#[derive(Debug, Clone, Copy)]
pub struct HexColor(pub Color);

impl Default for HexColor {
    fn default() -> Self {
        HexColor(Color::Reset)
    }
}

impl Serialize for HexColor {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        match self.0 {
            Color::Rgb(r, g, b) => s.serialize_str(&format!("#{r:02X}{g:02X}{b:02X}")),
            _ => s.serialize_str("#FFFFFF"),
        }
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(HexColor(parse_hex_color(&s).unwrap_or(Color::Reset)))
    }
}

// ─── Raw TOML structures (serde targets) ────────────────────────────────────
//
// Each struct carries `#[serde(default)]` so that missing keys or entire
// sections gracefully fill in from the compiled defaults.

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RawConfig {
    general: RawGeneral,
    bluetooth: RawBluetooth,
    notifications: RawNotifications,
    theme: RawTheme,
    keybindings: RawKeybindings,
}

impl Default for RawConfig {
    fn default() -> Self {
        Self {
            general: RawGeneral::default(),
            bluetooth: RawBluetooth::default(),
            notifications: RawNotifications::default(),
            theme: RawTheme::default(),
            keybindings: RawKeybindings::default(),
        }
    }
}

// ── General ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RawGeneral {
    tick_rate_ms: u64,
    scan_on_startup: bool,
    hide_unnamed_devices: bool,
    device_list_percent: u16,
}

impl Default for RawGeneral {
    fn default() -> Self {
        Self {
            tick_rate_ms: 16,
            scan_on_startup: false,
            hide_unnamed_devices: false,
            device_list_percent: 55,
        }
    }
}

// ── Bluetooth ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RawBluetooth {
    auto_trust_on_pair: bool,
    connection_timeout_secs: u64,
}

impl Default for RawBluetooth {
    fn default() -> Self {
        Self {
            auto_trust_on_pair: true,
            connection_timeout_secs: 30,
        }
    }
}

// ── Notifications ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RawNotifications {
    success_duration_ms: u64,
    error_duration_ms: u64,
    slide_speed: f32,
}

impl Default for RawNotifications {
    fn default() -> Self {
        Self {
            success_duration_ms: 3000,
            error_duration_ms: 7000,
            slide_speed: 0.08,
        }
    }
}

// ── Theme ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct RawTheme {
    palette: RawPalette,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RawPalette {
    accent_primary: HexColor,
    accent_secondary: HexColor,
    accent_error: HexColor,
    text_primary: HexColor,
    text_dim: HexColor,
    paired: HexColor,
    success: HexColor,
    scanning: HexColor,
    border_inactive: HexColor,
}

impl Default for RawPalette {
    fn default() -> Self {
        Self {
            accent_primary: HexColor(Color::Rgb(120, 220, 255)),
            accent_secondary: HexColor(Color::Rgb(180, 160, 255)),
            accent_error: HexColor(Color::Rgb(255, 140, 160)),
            text_primary: HexColor(Color::Rgb(225, 223, 240)),
            text_dim: HexColor(Color::Rgb(120, 124, 150)),
            paired: HexColor(Color::Rgb(255, 200, 120)),
            success: HexColor(Color::Rgb(130, 235, 175)),
            scanning: HexColor(Color::Rgb(100, 230, 255)),
            border_inactive: HexColor(Color::Rgb(140, 143, 165)),
        }
    }
}

// ── Keybindings ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RawKeybindings {
    quit: String,
    nav_down: String,
    nav_up: String,
    jump_top: String,
    jump_bottom: String,
    search: String,
    help: String,
    toggle_adapter: String,
    toggle_scan: String,
    connect_toggle: String,
    disconnect: String,
    pair: String,
    trust: String,
    remove: String,
    refresh: String,
}

impl Default for RawKeybindings {
    fn default() -> Self {
        Self {
            quit: "q".into(),
            nav_down: "j".into(),
            nav_up: "k".into(),
            jump_top: "g".into(),
            jump_bottom: "G".into(),
            search: "/".into(),
            help: "?".into(),
            toggle_adapter: "a".into(),
            toggle_scan: "s".into(),
            connect_toggle: "Enter".into(),
            disconnect: "d".into(),
            pair: "p".into(),
            trust: "t".into(),
            remove: "r".into(),
            refresh: "R".into(),
        }
    }
}

// ─── Resolved runtime config ────────────────────────────────────────────────
//
// These are the structs the rest of the app interacts with.  All values are
// validated, clamped, and ready to use — no further parsing at render time.

/// Fully resolved, runtime-ready configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub bluetooth: BluetoothConfig,
    pub notifications: NotificationsConfig,
    pub theme: ThemeConfig,
    pub keys: KeybindingsConfig,
}

#[derive(Debug, Clone)]
pub struct GeneralConfig {
    pub tick_rate_ms: u64,
    pub scan_on_startup: bool,
    pub hide_unnamed_devices: bool,
    pub device_list_percent: u16,
}

#[derive(Debug, Clone)]
pub struct BluetoothConfig {
    pub auto_trust_on_pair: bool,
    pub connection_timeout_secs: u64,
}

#[derive(Debug, Clone)]
pub struct NotificationsConfig {
    pub success_duration_ms: u64,
    pub error_duration_ms: u64,
    pub slide_speed: f32,
}

#[derive(Debug, Clone)]
pub struct ThemeConfig {
    pub palette: Palette,
}

/// Resolved colour palette — every field is a ready-to-use `Color`.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub accent_primary: Color,
    pub accent_secondary: Color,
    pub accent_error: Color,
    pub text_primary: Color,
    pub text_dim: Color,
    pub paired: Color,
    pub success: Color,
    pub scanning: Color,
    pub border_inactive: Color,
}

/// Pre-parsed keybindings — each field is a `KeyCode` ready for matching.
#[derive(Debug, Clone)]
pub struct KeybindingsConfig {
    pub quit: KeyCode,
    pub nav_down: KeyCode,
    pub nav_up: KeyCode,
    pub jump_top: KeyCode,
    pub jump_bottom: KeyCode,
    pub search: KeyCode,
    pub help: KeyCode,
    pub toggle_adapter: KeyCode,
    pub toggle_scan: KeyCode,
    pub connect_toggle: KeyCode,
    pub disconnect: KeyCode,
    pub pair: KeyCode,
    pub trust: KeyCode,
    pub remove: KeyCode,
    pub refresh: KeyCode,
}

// ─── Raw → Resolved conversion ─────────────────────────────────────────────

impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        Self {
            general: GeneralConfig {
                tick_rate_ms: raw.general.tick_rate_ms.clamp(4, 200),
                scan_on_startup: raw.general.scan_on_startup,
                hide_unnamed_devices: raw.general.hide_unnamed_devices,
                device_list_percent: raw.general.device_list_percent.clamp(20, 80),
            },
            bluetooth: BluetoothConfig {
                auto_trust_on_pair: raw.bluetooth.auto_trust_on_pair,
                connection_timeout_secs: raw.bluetooth.connection_timeout_secs.clamp(5, 120),
            },
            notifications: NotificationsConfig {
                success_duration_ms: raw.notifications.success_duration_ms.clamp(500, 30_000),
                error_duration_ms: raw.notifications.error_duration_ms.clamp(500, 60_000),
                slide_speed: raw.notifications.slide_speed.clamp(0.01, 1.0),
            },
            theme: ThemeConfig {
                palette: Palette {
                    accent_primary: raw.theme.palette.accent_primary.0,
                    accent_secondary: raw.theme.palette.accent_secondary.0,
                    accent_error: raw.theme.palette.accent_error.0,
                    text_primary: raw.theme.palette.text_primary.0,
                    text_dim: raw.theme.palette.text_dim.0,
                    paired: raw.theme.palette.paired.0,
                    success: raw.theme.palette.success.0,
                    scanning: raw.theme.palette.scanning.0,
                    border_inactive: raw.theme.palette.border_inactive.0,
                },
            },
            keys: KeybindingsConfig {
                quit: parse_key(&raw.keybindings.quit),
                nav_down: parse_key(&raw.keybindings.nav_down),
                nav_up: parse_key(&raw.keybindings.nav_up),
                jump_top: parse_key(&raw.keybindings.jump_top),
                jump_bottom: parse_key(&raw.keybindings.jump_bottom),
                search: parse_key(&raw.keybindings.search),
                help: parse_key(&raw.keybindings.help),
                toggle_adapter: parse_key(&raw.keybindings.toggle_adapter),
                toggle_scan: parse_key(&raw.keybindings.toggle_scan),
                connect_toggle: parse_key(&raw.keybindings.connect_toggle),
                disconnect: parse_key(&raw.keybindings.disconnect),
                pair: parse_key(&raw.keybindings.pair),
                trust: parse_key(&raw.keybindings.trust),
                remove: parse_key(&raw.keybindings.remove),
                refresh: parse_key(&raw.keybindings.refresh),
            },
        }
    }
}

/// Parse a human-readable key name into a crossterm `KeyCode`.
fn parse_key(s: &str) -> KeyCode {
    match s {
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Space" => KeyCode::Char(' '),
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "Delete" => KeyCode::Delete,
        "Insert" => KeyCode::Insert,
        s if s.len() == 1 => KeyCode::Char(s.chars().next().unwrap()),
        other => {
            warn!("Unknown keybinding \"{other}\" in config — ignoring");
            KeyCode::Null
        }
    }
}
