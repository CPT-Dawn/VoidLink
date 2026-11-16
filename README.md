<div align="center">

<img src="assets/logo.svg" width="96" alt="VoidLink logo">

# VoidLink

**Bluetooth, without leaving your terminal.**

A keyboard-first Bluetooth manager for Linux. Scan, pair and connect from one<br>
fast TUI that talks to BlueZ directly, and asks you before any device pairs.

[![Release](https://img.shields.io/github/v/release/CPT-Dawn/VoidLink?style=flat-square)](https://github.com/CPT-Dawn/VoidLink/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/CPT-Dawn/VoidLink/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/CPT-Dawn/VoidLink/actions/workflows/ci.yml)
[![AUR](https://img.shields.io/aur/version/voidlink-bin?style=flat-square&logo=archlinux&logoColor=white&label=AUR)](https://aur.archlinux.org/packages/voidlink-bin)
[![License](https://img.shields.io/github/license/CPT-Dawn/VoidLink?style=flat-square)](LICENSE)

[Install](#install) · [Quick start](#quick-start) · [Documentation](docs/README.md) · [Contributing](CONTRIBUTING.md)

<img src="assets/demo.gif" width="100%" alt="VoidLink scanning for devices, searching for a speaker, confirming its pairing code, renaming it and opening the help screen">

<sub><a href="assets/demo.mp4">Watch in full quality</a></sub>

</div>

## Why VoidLink

- **Pairing asks you first.** Every request shows its code or asks for a PIN, and an unanswered prompt is rejected after 24 seconds.
- **Idle means idle.** VoidLink redraws only when something changes. With nothing animating, it doesn't wake at all.
- **Always current.** Connection, battery and signal update live, even when another app or the device makes the change.
- **Errors come with the fix.** An rfkill block, a stopped `bluetoothd` or a missing audio stack each tells you the command that solves it.
- **One small binary.** About 4.8 MB, talking to BlueZ over D-Bus with no `bluetoothctl` underneath.

## Install

On Arch Linux, from the AUR:

```sh
paru -S voidlink-bin
```

<details>
<summary>Other ways to install</summary>

**AUR, built from source**

```sh
paru -S voidlink
```

**Prebuilt binary** (x86_64 or aarch64), from the [latest release](https://github.com/CPT-Dawn/VoidLink/releases/latest)

```sh
tar -xzf voidlink-*-linux.tar.gz
sudo install -Dm755 voidlink-*-linux/voidlink /usr/local/bin/voidlink
```

**From source** (needs Rust 1.88+ and the D-Bus headers)

```sh
cargo install --locked --git https://github.com/CPT-Dawn/VoidLink
```

</details>

VoidLink needs BlueZ with its service running: `sudo systemctl enable --now bluetooth.service`.
[Installation](docs/installation.md) has the per-distro packages, including audio and fonts.

## Quick start

```sh
voidlink
```

1. Press <kbd>s</kbd> to scan. Nearby devices appear under **Nearby**.
2. Put your device in pairing mode, select it and press <kbd>Enter</kbd>.
3. If a code appears, check it matches the device and press <kbd>y</kbd>.

VoidLink pairs, trusts and connects in one step, and the device moves to **My devices**.
Press <kbd>?</kbd> for every key, or run `voidlink --help` for the flags.

## Documentation

| Guide | What it covers |
| --- | --- |
| [Installation](docs/installation.md) | Every channel, runtime packages per distro, building from source |
| [Configuration](docs/configuration.md) | Every option and its default, colours, command-line flags |
| [Keybindings](docs/keybindings.md) | Every key and the mouse, and how to remap them |
| [Troubleshooting](docs/troubleshooting.md) | The errors people hit, each with its fix |
| [Architecture](docs/architecture.md) | How the UI, the Bluetooth worker and BlueZ fit together |
| [Changelog](CHANGELOG.md) | What changed in each release |

## Contributing

Issues and pull requests are welcome, and you don't need Bluetooth hardware to work on the UI.
[CONTRIBUTING.md](CONTRIBUTING.md) has the setup, the checks CI runs and how the demo is recorded.

## License

[MIT](LICENSE) © Swastik Patel ([CPT-Dawn](https://github.com/CPT-Dawn))
