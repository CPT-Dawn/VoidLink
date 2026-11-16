# Installation

Every way to install VoidLink, and the system packages it needs at runtime.

## Arch Linux (AUR)

| Package | What you get |
| --- | --- |
| [`voidlink-bin`](https://aur.archlinux.org/packages/voidlink-bin) | The prebuilt release binary; installs in seconds |
| [`voidlink`](https://aur.archlinux.org/packages/voidlink) | Built from source on your machine |

```sh
paru -S voidlink-bin
```

`yay -S …` works the same way.

## Prebuilt binary, any distribution

Each [release](https://github.com/CPT-Dawn/VoidLink/releases/latest) has an archive for `x86_64` and `aarch64`, with a `.sha256` checksum beside it.

1. Download `voidlink-<version>-<arch>-linux.tar.gz` and its `.sha256`.
2. Check it: `sha256sum -c voidlink-*-linux.tar.gz.sha256`
3. Unpack and install:

```sh
tar -xzf voidlink-*-linux.tar.gz
sudo install -Dm755 voidlink-*-linux/voidlink /usr/local/bin/voidlink
```

## From source

You need Rust 1.88 or newer, `pkg-config` and the D-Bus development headers (`libdbus-1-dev` on Debian and Ubuntu, `dbus-devel` on Fedora, `dbus` on Arch).

```sh
cargo install --locked --git https://github.com/CPT-Dawn/VoidLink
```

## Runtime requirements

| What | Why | Arch | Debian / Ubuntu | Fedora |
| --- | --- | --- | --- | --- |
| BlueZ, service running | The Bluetooth stack VoidLink drives | `bluez` | `bluez` | `bluez` |
| D-Bus | How VoidLink talks to BlueZ | `dbus` | `dbus` | `dbus` |
| Nerd Font *(optional)* | Device and status icons | `ttf-nerd-fonts-symbols-mono` | [nerdfonts.com](https://www.nerdfonts.com) | [nerdfonts.com](https://www.nerdfonts.com) |
| Bluetooth audio *(optional)* | Headphones and speakers | `pipewire-audio` | `libspa-0.2-bluetooth` | included with PipeWire |

Enable the Bluetooth service once:

```sh
sudo systemctl enable --now bluetooth.service
```

Without a Nerd Font, run `voidlink --no-nerd-font` or set `nerd_font = false` in the [configuration](configuration.md).

[Back to the README](../README.md)
