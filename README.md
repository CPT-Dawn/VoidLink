# VoidLink

**VoidLink** is a high-performance, keyboard-first Bluetooth manager for Linux terminals. Engineered in Rust for modern Wayland workflows, it provides a sleek, memory-safe interface to the BlueZ stack via D-Bus, eliminating the overhead of subprocess-heavy alternatives.

[![Rust](https://img.shields.io/badge/Rust-1.75%2B-f74c00?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![AUR](https://img.shields.io/aur/version/voidlink?style=flat-square&color=0891b2&label=AUR)](https://aur.archlinux.org/packages/voidlink)
[![License: MIT](https://img.shields.io/badge/License-MIT-2ea043?style=flat-square)](LICENSE)
[![Maintenance](https://img.shields.io/badge/Maintained-actively-1f883d?style=flat-square)](https://github.com/cptdawn/VoidLink)

---

## Why VoidLink?

Legacy Bluetooth tools often rely on parsing the output of interactive shells, leading to fragile behavior and high resource churn. VoidLink takes a different path:

- **Native D-Bus Integration:** Talks directly to BlueZ using the `bluer` crate. No `bluetoothctl` subprocesses.
- **Async Architecture:** A dedicated Tokio worker handles the Bluetooth stack, ensuring the UI remains snappy and responsive.
- **Zero-Polling:** Redraws are strictly event-driven. The UI only wakes up when your adapter or devices actually change.
- **Keyboard-First:** Every action is a keystroke away, designed to fit perfectly into tiling window manager workflows (Hyprland, Sway, etc.).

## Features

- **Full Lifecycle Management:** Power, scan, pair, trust, connect, and remove devices with ease.
- **Custom Bluetooth Agent:** Seamlessly handles PIN and passkey forwarding directly within the TUI.
- **Live Search & Filter:** Instant device filtering using plain text, smart regex, or substring matching.
- **Deep Device Insights:** Detailed panel showing RSSI (Signal), Battery level, Device Class, and Connection status with live gauges.
- **Persistent Configuration:** Fully customizable keybindings and color palettes via `~/.config/voidlink/config.toml`.
- **Modern TUI:** Built with `ratatui` and Nerd Fonts for a beautiful, professional terminal experience.

## Installation

### Arch Linux (AUR)
VoidLink is available on the AUR. You can install it using your favorite AUR helper:

```bash
paru -S voidlink
# or
yay -S voidlink
```

### Build from Source
Ensure you have the Rust toolchain installed.

```bash
git clone https://github.com/cptdawn/VoidLink.git
cd VoidLink
cargo build --release
install -Dm755 target/release/voidlink -t ~/.local/bin/
```

## Keybindings

VoidLink uses intuitive, Vim-like defaults. All keys are remappable in the config file.

| Key | Action |
| :--- | :--- |
| `j` / `k` | Move selection (or `↑`/`↓`) |
| `Enter` | Toggle Connection (Connect/Disconnect) |
| `p` | Initiate Pairing |
| `t` | Toggle Trust Status |
| `r` | Remove/Forget Device |
| `a` | Toggle Adapter Power |
| `s` | Start/Stop Scanning |
| `S` | Cycle Sort Mode (Name, RSSI, Address, Default) |
| `/` | Search Mode (prefix with `/` for Regex) |
| `A` | Rename Device (Alias) |
| `?` | Show Help Overlay |
| `q` | Quit |

## Configuration

On first run, VoidLink generates a default configuration at `~/.config/voidlink/config.toml`.

```toml
[general]
tick_rate_ms = 16
scan_on_startup = false
device_list_percent = 55
sort_mode = "default"      # default | name | rssi | address
search_mode = "smart"      # smart | plain | regex

[theme.palette]
accent_primary = "#0891b2"    # Plasma Cyan
accent_secondary = "#8b5cf6"  # Nebula Violet
accent_error = "#ef4444"      # Dawn Red
text_primary = "#f8fafc"      # Starlight
text_dim = "#94a3b8"          # Cosmic Dust
```

## Architecture

VoidLink follows a strict separation of concerns:

- **Main Thread:** Manages the `ratatui` render loop and processes user input.
- **Bluetooth Worker:** A background Tokio task that owns the BlueZ session and adapter handles.
- **Communication:** Bounded `mpsc` channels ensure safe, asynchronous communication between the UI and system layers.

## Contributing

We welcome contributions! Please feel free to open issues or submit pull requests.

1. Fork the repo.
2. Create your feature branch (`git checkout -b feat/new-feature`).
3. Commit your changes.
4. Push to the branch.
5. Open a Pull Request.

## License

VoidLink is released under the [MIT License](LICENSE).

---

*Crafted with 󰄛 by CPTDawn.*
