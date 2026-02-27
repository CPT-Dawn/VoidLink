# VOIDLINK

A memory-safe, keyboard-first Bluetooth manager for Linux terminals in modern Wayland workflows.

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-f74c00?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-2ea043?style=flat-square)](LICENSE)
[![Wayland](https://img.shields.io/badge/Wayland-native%20workflow-7d8590?style=flat-square)](https://wayland.freedesktop.org)
[![Maintenance](https://img.shields.io/badge/Maintained-actively-1f883d?style=flat-square)](https://github.com/cptdawn/VoidLink)

![Demo](assets/demo.gif)

> [!NOTE]
> VoidLink does **not** speak Wayland protocols directly; it is a terminal UI. It is compositor-agnostic (including Hyprland, Sway, GNOME, KDE) and talks to Bluetooth through BlueZ on the system D-Bus.

## Why

Most legacy Bluetooth interfaces are wrappers over interactive shell tooling, with subprocess churn and fragile text parsing. VoidLink is engineered as a direct, typed system client: an async Rust worker owns BlueZ communication over D-Bus, while the UI loop remains isolated and deterministic.

This architecture keeps the binary small, avoids polling loops, and preserves strict ownership boundaries between terminal rendering and transport logic. The result is predictable behavior under load, strong memory safety guarantees, and clean UNIX-style separation of concerns.

## Features

- Direct BlueZ integration over system D-Bus via `bluer` (no `bluetoothctl` subprocess layer)
- Event-driven worker model with bounded `tokio::mpsc` channels (`BtCommand` and `BtEvent`)
- Zero-polling UI path: redraws are dirty-flag driven; adapter/device updates are signal-based
- Full lifecycle operations: power, scan, pair, trust toggle, connect/disconnect, remove, alias rename
- Custom BlueZ Agent implementation for passkey/PIN forwarding into the TUI
- Configurable connect lifecycle (`pair -> trust -> connect`) with timeout controls
- Runtime-sortable device list (`default`, `name`, `rssi`, `address`) and live search (`plain`/`regex`/`smart`)
- Embedded default config bootloader with first-run materialization to XDG config directory
- Terminal-safe lifecycle management (raw mode + alternate screen restore on panic)

## Installation

### Build from Source

```bash
git clone https://github.com/cptdawn/VoidLink.git
cd VoidLink
cargo build --release
```

Run the binary:

```bash
./target/release/voidlink
```

### Arch Linux (AUR)

```bash
yay -S <project-name>
```

## Configuration

VoidLink uses an embedded asset bootloader pattern:

1. `default_config.toml` is embedded at compile time via `include_str!`.
2. On first launch, the app creates the user config file and writes embedded defaults.
3. On subsequent launches, user TOML is parsed; missing fields fall back to defaults with `serde` defaults.
4. The resolved runtime config is stored in a global `OnceLock` and accessed as an immutable singleton.

Generated path:

```text
~/.config/voidlink/config.toml
```

Example:

```toml
[general]
tick_rate_ms = 16
scan_on_startup = false
hide_unnamed_devices = false
device_list_percent = 55
sort_mode = "default"      # default | name | rssi | address
search_mode = "smart"      # smart | plain | regex

[bluetooth]
auto_trust_on_pair = true
connection_timeout_secs = 30

[notifications]
success_duration_ms = 3000
error_duration_ms = 7000
slide_speed = 0.08

[keybindings]
quit = "q"
nav_down = "j"
nav_up = "k"
search = "/"
toggle_adapter = "a"
toggle_scan = "s"
connect_toggle = "Enter"
pair = "p"
trust = "t"
disconnect = "d"
remove = "r"
refresh = "R"
cycle_sort = "S"
rename = "A"
```

Key groups:

- `[general]`: render cadence, startup behavior, list layout, sorting/search semantics
- `[bluetooth]`: trust automation and connection timeout envelope
- `[notifications]`: popup timing and animation rate
- `[theme.palette]`: color tokens consumed by the TUI theme layer
- `[keybindings]`: remappable keycodes for all major actions

## Usage

Start VoidLink:

```bash
voidlink
```

If not installed globally:

```bash
cargo run --release
```

Core shortcuts:

| Key | Action |
| --- | --- |
| `j` / `k` or `↑` / `↓` | Move selection |
| `g` / `G` | Jump top / bottom |
| `a` | Toggle adapter power |
| `s` | Start/stop discovery |
| `Enter` | Connect/disconnect selected device |
| `p` | Pair selected device |
| `t` | Toggle trust |
| `d` | Disconnect |
| `r` | Remove/forget device |
| `R` | Refresh selected device snapshot |
| `A` | Set alias (rename) |
| `S` | Cycle sort mode |
| `/` | Search mode (smart regex if prefixed with `/`) |
| `?` | Help overlay |
| `q` or `Ctrl+C` | Quit |

## Architecture

```text
UI thread (ratatui + crossterm)
  ├─ owns App state and render loop
  ├─ processes keyboard/resize/tick events
  └─ sends BtCommand over bounded mpsc

Tokio Bluetooth worker
  ├─ owns bluer::Session + default Adapter
  ├─ registers custom BlueZ Agent callbacks
  ├─ consumes BtCommand and executes BlueZ operations
  └─ emits BtEvent snapshots/results to UI
```

Protocol stack in use:

- Bluetooth control plane: BlueZ over system D-Bus
- Rust access layer: `bluer` crate
- Terminal frontend: `ratatui` + `crossterm`

## Contributing

Contributions are welcome. Please open an issue for substantial changes before submitting a PR.

1. Fork the repository
2. Create a branch: `git checkout -b feat/<topic>`
3. Build and test locally: `cargo build --release`
4. Submit a focused pull request with a clear rationale

## License

Licensed under the MIT License. See [LICENSE](LICENSE).