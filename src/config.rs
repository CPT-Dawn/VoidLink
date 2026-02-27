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
///
/// # Panics
/// Panics only if `init()` was never called — a programmer error that cannot
/// occur during normal operation since `main()` calls `init()` before any
/// other module.
#[inline]
pub fn get() -> &'static Config {
    CONFIG.get().expect("config::init() was not called")
}

// ─── Loading logic ──────────────────────────────────────────────────────────

fn load() -> Result<Config> {
    let defaults: RawConfig = toml::from_str(DEFAULT_CONFIG_STR)
        .wrap_err("BUG: failed to parse embedded default_config.toml")?;

    let user_path = config_path();
    info!("Config path: {}", user_path.display());

    ensure_config_file(&user_path)?;

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

fn config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "voidlink")
        .map(|dirs| dirs.config_dir().join("config.toml"))
        .unwrap_or_else(|| PathBuf::from(".config/voidlink/config.toml"))
}

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
///
/// Operates on ASCII bytes to avoid multi-byte UTF-8 slice panics.
fn parse_hex_color(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    let bytes = hex.as_bytes();
    if bytes.len() != 6 || !bytes.iter().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

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

// ─── Public enums ───────────────────────────────────────────────────────────

/// Device sort order — runtime-cyclable via keybinding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortMode {
    #[default]
    Default,
    Name,
    Rssi,
    Address,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::Name,
            Self::Name => Self::Rssi,
            Self::Rssi => Self::Address,
            Self::Address => Self::Default,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Name => "Name",
            Self::Rssi => "RSSI",
            Self::Address => "Address",
        }
    }
}

/// Search matching mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Smart,
    Plain,
    Regex,
}

// ─── Raw TOML structures (serde targets) ────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct RawConfig {
    general: RawGeneral,
    bluetooth: RawBluetooth,
    notifications: RawNotifications,
    theme: RawTheme,
    keybindings: RawKeybindings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RawGeneral {
    tick_rate_ms: u64,
    scan_on_startup: bool,
    hide_unnamed_devices: bool,
    device_list_percent: u16,
    sort_mode: String,
    search_mode: String,
}

impl Default for RawGeneral {
    fn default() -> Self {
        Self {
            tick_rate_ms: 16,
            scan_on_startup: false,
            hide_unnamed_devices: false,
            device_list_percent: 55,
            sort_mode: "default".into(),
            search_mode: "smart".into(),
        }
    }
}

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
        // Cosmic Dawn — high-contrast palette for translucent terminals.
        Self {
            accent_primary: HexColor(Color::Rgb(0x00, 0xE5, 0xFF)),
            accent_secondary: HexColor(Color::Rgb(0xBB, 0x86, 0xFC)),
            accent_error: HexColor(Color::Rgb(0xFF, 0x45, 0x45)),
            text_primary: HexColor(Color::Rgb(0xE8, 0xE6, 0xF0)),
            text_dim: HexColor(Color::Rgb(0x6B, 0x6F, 0x85)),
            paired: HexColor(Color::Rgb(0xFF, 0xB7, 0x4D)),
            success: HexColor(Color::Rgb(0x69, 0xF0, 0xAE)),
            scanning: HexColor(Color::Rgb(0x18, 0xFF, 0xFF)),
            border_inactive: HexColor(Color::Rgb(0x4A, 0x4E, 0x69)),
        }
    }
}

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
    cycle_sort: String,
    rename: String,
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
            cycle_sort: "S".into(),
            rename: "A".into(),
        }
    }
}

// ─── Resolved runtime config ────────────────────────────────────────────────

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
    pub sort_mode: SortMode,
    pub search_mode: SearchMode,
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
    pub cycle_sort: KeyCode,
    pub rename: KeyCode,
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
                sort_mode: match raw.general.sort_mode.as_str() {
                    "name" => SortMode::Name,
                    "rssi" => SortMode::Rssi,
                    "address" => SortMode::Address,
                    _ => SortMode::Default,
                },
                search_mode: match raw.general.search_mode.as_str() {
                    "plain" => SearchMode::Plain,
                    "regex" => SearchMode::Regex,
                    _ => SearchMode::Smart,
                },
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
                cycle_sort: parse_key(&raw.keybindings.cycle_sort),
                rename: parse_key(&raw.keybindings.rename),
            },
        }
    }
}

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
        s if s.len() == 1 => {
            // Safe: single-byte UTF-8 strings always yield one char.
            s.chars().next().map_or(KeyCode::Null, KeyCode::Char)
        }
        other => {
            warn!("Unknown keybinding \"{other}\" in config — ignoring");
            KeyCode::Null
        }
    }
}
