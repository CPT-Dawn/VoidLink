//! Configuration system with embedded defaults and XDG-compliant paths.
//!
//! Boot sequence:
//! 1. Resolve `~/.config/voidlink/config.toml` via the `directories` crate.
//! 2. If the file doesn't exist, write the embedded `default_config.toml` there
//!    (best effort — a read-only home is fine).
//! 3. Parse it. Every key is optional; anything missing or invalid falls back
//!    to the compiled-in default and produces a warning the UI shows once.
//! 4. Store the resolved `Config` in a `OnceLock` for zero-cost global access.
//!
//! Every other module calls `config::get()` to obtain a `&'static Config`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crossterm::event::KeyCode;
use ratatui::style::Color;
use serde::Deserialize;
use tracing::{info, warn};

/// Embedded default configuration — baked into the binary at compile time.
const DEFAULT_CONFIG_STR: &str = include_str!("../default_config.toml");

/// Application-wide config singleton.
static CONFIG: OnceLock<Config> = OnceLock::new();

// ─── Public API ─────────────────────────────────────────────────────────────

/// Initialise the configuration system. Must be called exactly once at
/// startup, before any other module calls `get()`.
///
/// Never fails: problems (unreadable file, bad TOML, unknown keys or colours)
/// fall back to defaults and are returned as user-facing warnings.
pub fn init() -> Vec<String> {
    let (config, warnings) = load();
    let _ = CONFIG.set(config);
    warnings
}

/// Return a static reference to the loaded configuration.
///
/// # Panics
/// Panics only if `init()` was never called — a programmer error.
#[inline]
pub fn get() -> &'static Config {
    CONFIG.get().expect("config::init() was not called")
}

/// The embedded default configuration file, as written on first launch.
pub fn default_config_str() -> &'static str {
    DEFAULT_CONFIG_STR
}

/// Where the user's config file lives.
pub fn config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "voidlink").map_or_else(
        || PathBuf::from(".config/voidlink/config.toml"),
        |dirs| dirs.config_dir().join("config.toml"),
    )
}

// ─── Loading logic ──────────────────────────────────────────────────────────

fn load() -> (Config, Vec<String>) {
    let mut warnings = Vec::new();
    let path = config_path();
    info!("Config path: {}", path.display());

    if let Err(e) = ensure_config_file(&path) {
        warn!("{e}");
    }

    let raw = match fs::read_to_string(&path) {
        Ok(contents) => parse(&contents, &mut warnings),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => RawConfig::default(),
        Err(e) => {
            warnings.push(format!(
                "Can't read {}: {e}. Using defaults",
                path.display()
            ));
            RawConfig::default()
        }
    };
    let config = Config::resolve(raw, &mut warnings);
    for w in &warnings {
        warn!("{w}");
    }
    (config, warnings)
}

/// Parse a config file; on a syntax error keep the defaults and warn.
fn parse(contents: &str, warnings: &mut Vec<String>) -> RawConfig {
    match toml::from_str::<RawConfig>(contents) {
        Ok(parsed) => parsed,
        Err(e) => {
            let detail = e.message().to_string();
            let line = e.span().map(|span| {
                contents[..span.start.min(contents.len())]
                    .lines()
                    .count()
                    .max(1)
            });
            warnings.push(match line {
                Some(line) => format!("Config error on line {line}: {detail}. Using defaults"),
                None => format!("Config error: {detail}. Using defaults"),
            });
            RawConfig::default()
        }
    }
}

fn ensure_config_file(path: &Path) -> std::result::Result<(), String> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Can't create {}: {e}", parent.display()))?;
    }
    fs::write(path, DEFAULT_CONFIG_STR)
        .map_err(|e| format!("Can't write default config to {}: {e}", path.display()))?;
    info!("Created default config at {}", path.display());
    Ok(())
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
            Self::Default => "Smart",
            Self::Name => "Name",
            Self::Rssi => "Signal",
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawConfig {
    general: RawGeneral,
    bluetooth: RawBluetooth,
    notifications: RawNotifications,
    theme: RawTheme,
    keybindings: RawKeybindings,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct RawGeneral {
    tick_rate_ms: u64,
    scan_on_startup: bool,
    hide_unnamed_devices: bool,
    device_list_percent: u16,
    sort_mode: String,
    search_mode: String,
    nerd_font: bool,
    mouse: bool,
}

impl Default for RawGeneral {
    fn default() -> Self {
        Self {
            tick_rate_ms: 33,
            scan_on_startup: false,
            hide_unnamed_devices: false,
            device_list_percent: 58,
            sort_mode: "default".into(),
            search_mode: "smart".into(),
            nerd_font: true,
            mouse: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct RawNotifications {
    success_duration_ms: u64,
    error_duration_ms: u64,
}

impl Default for RawNotifications {
    fn default() -> Self {
        Self {
            success_duration_ms: 3000,
            error_duration_ms: 7000,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawTheme {
    palette: RawPalette,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
struct RawPalette {
    accent_primary: String,
    accent_secondary: String,
    accent_error: String,
    text_primary: String,
    text_dim: String,
    paired: String,
    success: String,
    scanning: String,
    border_inactive: String,
}

impl Default for RawPalette {
    fn default() -> Self {
        // Cosmic Dawn — high-contrast palette for translucent terminals.
        Self {
            accent_primary: "#00E5FF".into(),
            accent_secondary: "#BB86FC".into(),
            accent_error: "#FF4545".into(),
            text_primary: "#E8E6F0".into(),
            text_dim: "#6B6F85".into(),
            paired: "#FFB74D".into(),
            success: "#69F0AE".into(),
            scanning: "#18FFFF".into(),
            border_inactive: "#4A4E69".into(),
        }
    }
}

/// Generates the raw keybinding struct, its defaults, the resolved struct and
/// the conversion, from one table — so adding an action is a one-line change.
macro_rules! keybindings {
    ($($field:ident = $default:literal),* $(,)?) => {
        #[derive(Debug, Clone, Deserialize)]
        #[serde(default)]
        struct RawKeybindings { $($field: String,)* }

        impl Default for RawKeybindings {
            fn default() -> Self { Self { $($field: $default.into(),)* } }
        }

        #[derive(Debug, Clone)]
        pub struct KeybindingsConfig { $(pub $field: KeyCode,)* }

        impl KeybindingsConfig {
            fn resolve(raw: &RawKeybindings, warnings: &mut Vec<String>) -> Self {
                let resolved = Self {
                    $($field: parse_key(&raw.$field).unwrap_or_else(|| {
                        warnings.push(format!(
                            "Unknown key \"{}\" for keybindings.{}; using \"{}\"",
                            raw.$field, stringify!($field), $default
                        ));
                        parse_key($default).unwrap_or(KeyCode::Null)
                    }),)*
                };
                let all = [$((stringify!($field), resolved.$field),)*];
                for (i, (name, key)) in all.iter().enumerate() {
                    if let Some((other, _)) = all[..i].iter().find(|(_, k)| k == key) {
                        warnings.push(format!(
                            "keybindings.{name} and keybindings.{other} share the key {}",
                            key_label(*key)
                        ));
                    }
                }
                resolved
            }
        }
    };
}

keybindings! {
    quit = "q",
    nav_down = "j",
    nav_up = "k",
    jump_top = "g",
    jump_bottom = "G",
    search = "/",
    help = "?",
    toggle_adapter = "a",
    toggle_scan = "s",
    connect_toggle = "Enter",
    disconnect = "d",
    pair = "p",
    trust = "t",
    remove = "r",
    refresh = "R",
    cycle_sort = "S",
    rename = "A",
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
    pub nerd_font: bool,
    pub mouse: bool,
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

impl Default for Config {
    fn default() -> Self {
        Self::resolve(RawConfig::default(), &mut Vec::new())
    }
}

// ─── Raw → Resolved conversion ─────────────────────────────────────────────

impl Config {
    fn resolve(raw: RawConfig, warnings: &mut Vec<String>) -> Self {
        let g = raw.general;
        let sort_mode = match g.sort_mode.as_str() {
            "default" => SortMode::Default,
            "name" => SortMode::Name,
            "rssi" => SortMode::Rssi,
            "address" => SortMode::Address,
            other => {
                warnings.push(format!(
                    "general.sort_mode \"{other}\" is not one of default, name, rssi, address"
                ));
                SortMode::Default
            }
        };
        let search_mode = match g.search_mode.as_str() {
            "smart" => SearchMode::Smart,
            "plain" => SearchMode::Plain,
            "regex" => SearchMode::Regex,
            other => {
                warnings.push(format!(
                    "general.search_mode \"{other}\" is not one of smart, plain, regex"
                ));
                SearchMode::Smart
            }
        };

        let defaults = RawPalette::default();
        let p = raw.theme.palette;
        let mut color = |name: &str, value: &str, fallback: &str| {
            parse_hex_color(value).unwrap_or_else(|| {
                warnings.push(format!(
                    "theme.palette.{name} \"{value}\" is not a #RRGGBB colour"
                ));
                parse_hex_color(fallback).unwrap_or(Color::Reset)
            })
        };
        let palette = Palette {
            accent_primary: color(
                "accent_primary",
                &p.accent_primary,
                &defaults.accent_primary,
            ),
            accent_secondary: color(
                "accent_secondary",
                &p.accent_secondary,
                &defaults.accent_secondary,
            ),
            accent_error: color("accent_error", &p.accent_error, &defaults.accent_error),
            text_primary: color("text_primary", &p.text_primary, &defaults.text_primary),
            text_dim: color("text_dim", &p.text_dim, &defaults.text_dim),
            paired: color("paired", &p.paired, &defaults.paired),
            success: color("success", &p.success, &defaults.success),
            scanning: color("scanning", &p.scanning, &defaults.scanning),
            border_inactive: color(
                "border_inactive",
                &p.border_inactive,
                &defaults.border_inactive,
            ),
        };

        Self {
            general: GeneralConfig {
                tick_rate_ms: g.tick_rate_ms.clamp(8, 200),
                scan_on_startup: g.scan_on_startup,
                hide_unnamed_devices: g.hide_unnamed_devices,
                device_list_percent: g.device_list_percent.clamp(30, 75),
                sort_mode,
                search_mode,
                nerd_font: g.nerd_font,
                mouse: g.mouse,
            },
            bluetooth: BluetoothConfig {
                auto_trust_on_pair: raw.bluetooth.auto_trust_on_pair,
                connection_timeout_secs: raw.bluetooth.connection_timeout_secs.clamp(5, 120),
            },
            notifications: NotificationsConfig {
                success_duration_ms: raw.notifications.success_duration_ms.clamp(500, 30_000),
                error_duration_ms: raw.notifications.error_duration_ms.clamp(500, 60_000),
            },
            theme: ThemeConfig { palette },
            keys: KeybindingsConfig::resolve(&raw.keybindings, warnings),
        }
    }
}

/// Parse a `#RRGGBB` hex string into an RGB `Color`.
fn parse_hex_color(s: &str) -> Option<Color> {
    let hex = s.trim().strip_prefix('#').unwrap_or(s.trim());
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some(Color::Rgb(channel(0)?, channel(2)?, channel(4)?))
}

/// Parse a key name from the config: a single character or a named key.
pub fn parse_key(s: &str) -> Option<KeyCode> {
    let key = match s {
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
        s => {
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => KeyCode::Char(c),
                _ => return None,
            }
        }
    };
    Some(key)
}

/// Short human label for a key, used in the hint bar and help screen.
pub fn key_label(key: KeyCode) -> String {
    match key {
        KeyCode::Enter => "Enter".into(),
        KeyCode::Esc => "Esc".into(),
        KeyCode::Tab => "Tab".into(),
        KeyCode::Backspace => "Bksp".into(),
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Up => "↑".into(),
        KeyCode::Down => "↓".into(),
        KeyCode::Left => "←".into(),
        KeyCode::Right => "→".into(),
        KeyCode::Home => "Home".into(),
        KeyCode::End => "End".into(),
        KeyCode::PageUp => "PgUp".into(),
        KeyCode::PageDown => "PgDn".into(),
        KeyCode::Delete => "Del".into(),
        KeyCode::Insert => "Ins".into(),
        other => format!("{other:?}"),
    }
}

/// Install the built-in defaults (unit tests never read the user's file).
#[cfg(test)]
pub fn init_for_tests() {
    let _ = CONFIG.set(Config::default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_str(toml: &str) -> (Config, Vec<String>) {
        let mut warnings = Vec::new();
        let raw = parse(toml, &mut warnings);
        let config = Config::resolve(raw, &mut warnings);
        (config, warnings)
    }

    #[test]
    fn embedded_default_config_is_valid_and_warning_free() {
        let (config, warnings) = load_str(DEFAULT_CONFIG_STR);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(config.keys.quit, KeyCode::Char('q'));
        assert_eq!(config.keys.disconnect, KeyCode::Char('d'));
        assert!(config.general.nerd_font);
    }

    #[test]
    fn empty_config_uses_defaults() {
        let (config, warnings) = load_str("");
        assert!(warnings.is_empty());
        assert_eq!(config.general.sort_mode, SortMode::Default);
        assert_eq!(config.keys.connect_toggle, KeyCode::Enter);
    }

    #[test]
    fn syntax_error_reports_line_and_falls_back() {
        let (config, warnings) = load_str("[general]\ntick_rate_ms = = 3\n");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("line 2"), "{}", warnings[0]);
        assert_eq!(config.general.tick_rate_ms, 33);
    }

    #[test]
    fn bad_values_warn_and_keep_defaults() {
        let (config, warnings) = load_str(
            "[general]\nsort_mode = \"wat\"\n[theme.palette]\naccent_primary = \"cyan\"\n[keybindings]\nquit = \"Ctrl+Q\"\n",
        );
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert_eq!(config.general.sort_mode, SortMode::Default);
        assert_eq!(
            config.theme.palette.accent_primary,
            Color::Rgb(0x00, 0xE5, 0xFF)
        );
        assert_eq!(config.keys.quit, KeyCode::Char('q'));
    }

    #[test]
    fn duplicate_keybindings_warn() {
        let (_, warnings) = load_str("[keybindings]\npair = \"t\"\n");
        assert!(
            warnings.iter().any(|w| w.contains("share the key t")),
            "{warnings:?}"
        );
    }

    #[test]
    fn old_configs_with_removed_keys_still_load() {
        let (_, warnings) = load_str("[notifications]\nslide_speed = 0.08\n");
        assert!(warnings.is_empty());
    }

    #[test]
    fn hex_colours() {
        assert_eq!(parse_hex_color("#0a0B0c"), Some(Color::Rgb(10, 11, 12)));
        assert_eq!(parse_hex_color("ffffff"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(parse_hex_color("#fff"), None);
        assert_eq!(parse_hex_color("#ééé"), None);
    }

    #[test]
    fn key_names() {
        assert_eq!(parse_key("Space"), Some(KeyCode::Char(' ')));
        assert_eq!(parse_key("é"), Some(KeyCode::Char('é')));
        assert_eq!(parse_key("ab"), None);
        assert_eq!(key_label(KeyCode::Enter), "Enter");
    }
}
