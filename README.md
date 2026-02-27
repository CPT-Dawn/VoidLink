<div align="center">

# VoidLink

**A zero-compromise Bluetooth manager for the terminal.**

Lightweight, keyboard-driven, and engineered for transparent terminals.

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-f74c00?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-78DCFF?style=flat-square)](LICENSE)
[![Linux/BlueZ](https://img.shields.io/badge/Linux-BlueZ%205.x-B4A0FF?style=flat-square&logo=linux&logoColor=white)](https://www.bluez.org)
[![Maintenance](https://img.shields.io/badge/Maintained-actively-82EBAF?style=flat-square)](https://github.com/cptdawn/VoidLink)

![Demo](assets/demo.gif)

</div>

---

## Philosophy

Most Bluetooth GUIs are bloated wrappers around `bluetoothctl` that shell out on every interaction, poll device state in hot loops, and fall apart when the adapter disappears mid-operation. VoidLink exists because managing Bluetooth should be fast, predictable, and invisible until you need it.

**Core tenets:**

- **D-Bus native.** VoidLink talks directly to BlueZ over D-Bus via [bluer](https://crates.io/crates/bluer) — no subprocess spawning, no `bluetoothctl` parsing, no shell injection surface.
- **Zero-polling architecture.** The UI thread never touches D-Bus. A dedicated async worker reacts to BlueZ property-change signals via `tokio::select!`, forwarding snapshots to the render loop through bounded channels.
- **Transparency-first design.** No solid backgrounds are ever rendered. Every style uses foreground color and modifier only, so your compositor blur, opacity, and wallpaper always shine through.
- **Single binary, zero runtime dependencies.** Statically links everything except `libdbus`. No Python, no Node, no config framework to install first.

> [!NOTE]
> VoidLink requires a running BlueZ daemon (`bluetoothd`) and D-Bus system bus.
> On most Linux distributions these are present out of the box — just ensure `bluetooth.service` is active.

## Features

- **Full device lifecycle** — scan, pair, trust, connect, disconnect, remove, all from one screen
- **Live adapter control** — toggle power and scanning without leaving the TUI
- **Real-time updates** — RSSI, battery percentage, and connection state stream in via D-Bus signals
- **Vim-style navigation** — `j`/`k` movement, `g`/`G` jump, `/` incremental search with live filtering
- **Fully remappable keybindings** — every action is configurable via TOML
- **Smart connect lifecycle** — pair → auto-trust → connect in one keypress, with configurable timeout protection
- **BlueZ agent integration** — PIN/passkey pairing prompts forwarded directly to the TUI
- **Sliding notification toasts** — animated popups for success/error events with configurable duration
- **Nerd Font device icons** — headphones, phones, keyboards, mice, speakers, gamepads — all mapped from BlueZ device class
- **RSSI signal gauge** — five-level signal bar with color-coded strength indicator
- **Battery monitoring** — live battery percentage display with tier-colored icons
- **Embedded configuration** — sensible defaults compiled into the binary; user overrides loaded from `~/.config/voidlink/config.toml`
- **Hide unnamed devices** — filter out address-only BLE advertisers cluttering the scan results
- **Adaptive popup sizing** — error dialogs scale to terminal width with proper text wrapping
- **Aggressive release binary** — LTO, symbol stripping, single codegen unit for minimal size

## Installation

### Build from source

```bash
# Clone
git clone https://github.com/cptdawn/VoidLink.git
cd VoidLink

# Build release binary
cargo build --release

# Install to ~/.cargo/bin
cargo install --path .
```

> [!IMPORTANT]
> Requires `libdbus-1-dev` (Debian/Ubuntu) or `dbus-devel` (Fedora) or `dbus` (Arch) as a build dependency.

### Arch Linux (AUR)

```bash
yay -S voidlink
```

## Configuration

VoidLink uses an **embedded asset** pattern for configuration:

1. A complete, heavily-commented `default_config.toml` is compiled into the binary via `include_str!`.
2. On first launch, VoidLink creates `~/.config/voidlink/config.toml` populated with those defaults.
3. On subsequent launches, the user file is deserialized first, then any missing keys fall back to the compiled-in defaults via `#[serde(default)]`.

This means VoidLink **always works out of the box** — no config file required — but every parameter is overridable.

### Config location

```
~/.config/voidlink/config.toml
```

### Example

```toml
[general]
tick_rate_ms = 16           # Render rate (~60 FPS). Range: 4–200
scan_on_startup = false     # Auto-scan when VoidLink launches
hide_unnamed_devices = false # Filter address-only BLE entries
device_list_percent = 55    # Device list pane width (%). Range: 20–80

[bluetooth]
auto_trust_on_pair = true   # Trust device automatically after pairing
connection_timeout_secs = 30 # Abort connect lifecycle after N seconds

[notifications]
success_duration_ms = 3000  # How long success toasts stay visible
error_duration_ms = 7000    # How long error toasts stay visible
slide_speed = 0.08          # Popup slide-in speed per tick (0.01–1.0)

[theme.palette]
accent_primary = "#78DCFF"    # Headers, active borders, connected
accent_secondary = "#B4A0FF"  # Selection highlight, focused elements
accent_error = "#FF8CA0"      # Error / destructive actions
text_primary = "#E1DFF0"      # Body text
text_dim = "#787C96"          # Secondary / dimmed text
paired = "#FFC878"            # Paired-but-not-connected indicator
success = "#82EBAF"           # Trusted / success indicator
scanning = "#64E6FF"          # Scanning spinner pulse
border_inactive = "#8C8FA5"   # Inactive pane borders

[keybindings]
quit = "q"
nav_down = "j"
nav_up = "k"
jump_top = "g"
jump_bottom = "G"
search = "/"
help = "?"
toggle_adapter = "a"
toggle_scan = "s"
connect_toggle = "Enter"
disconnect = "d"
pair = "p"
trust = "t"
remove = "r"
refresh = "R"
```

## Usage

### Keyboard shortcuts

| Key | Action |
|-----|--------|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `g` | Jump to top |
| `G` | Jump to bottom |
| `/` | Enter search mode (live filter) |
| `Enter` | Connect / disconnect selected device |
| `s` | Toggle scanning |
| `a` | Toggle adapter power |
| `p` | Pair with selected device |
| `t` | Trust selected device |
| `d` | Disconnect selected device |
| `r` | Remove (unpair) selected device |
| `R` | Refresh selected device info |
| `?` | Toggle help overlay |
| `q` | Quit |
| `Ctrl+C` | Force quit (always active) |
| `Esc` | Close search / dismiss dialog |

> Arrow keys always work for navigation regardless of keybinding configuration.

### Search mode

Press `/` to enter incremental search. The device list filters in real-time as you type. Press `Enter` to lock the filter and return to normal mode, or `Esc` to clear and exit.

## Architecture

```
┌─────────────────────────────────────────────┐
│              Main Thread (TUI)              │
│                                             │
│  crossterm events ──► App state ──► ratatui │
│        ▲                    │               │
│        │              BtCommand             │
│    BtEvent           (mpsc tx)              │
│   (mpsc rx)               │                │
│        │                  ▼                 │
│  ┌─────────────────────────────────────┐    │
│  │        Tokio Worker Task            │    │
│  │                                     │    │
│  │  bluer Session ◄──► BlueZ (D-Bus)  │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
```

The UI thread owns all rendering and input state. The Bluetooth worker owns the `bluer::Session` and `Adapter`, communicating exclusively through typed `BtCommand`/`BtEvent` enums over bounded `mpsc` channels. Neither thread blocks the other.

## Contributing

Contributions are welcome. Please open an issue first to discuss what you'd like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feat/my-feature`)
3. Commit your changes (`git commit -m 'feat: add my feature'`)
4. Push to the branch (`git push origin feat/my-feature`)
5. Open a Pull Request

## License

[MIT](LICENSE) © Swastik Patel