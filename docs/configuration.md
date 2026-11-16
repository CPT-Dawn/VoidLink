# Configuration

Every setting VoidLink reads, from `~/.config/voidlink/config.toml` (or `$XDG_CONFIG_HOME/voidlink/config.toml`). The source of truth is [`default_config.toml`](../default_config.toml), which VoidLink writes there on first launch.

- Every key is optional; a missing key takes its default.
- An invalid value, unknown key or bad colour is reported inside VoidLink and replaced by its default.
- `voidlink --print-default-config` prints a fresh copy; `voidlink --config-path` prints where the file lives.

## `[general]`

| Key | Default | What it does |
| --- | --- | --- |
| `tick_rate_ms` | `33` | Animation frame interval while something moves (spinners, toasts). Range 8–200. Idle means no redraws at all |
| `scan_on_startup` | `false` | Start scanning as soon as VoidLink opens |
| `hide_unnamed_devices` | `false` | Hide nearby devices with no name (anonymous BLE beacons). Paired devices always show |
| `device_list_percent` | `58` | Width of the device list on wide terminals, in percent. Range 30–75 |
| `sort_mode` | `"default"` | `"default"` (connected, then paired, then signal), `"name"`, `"rssi"` or `"address"`. Cycle with <kbd>S</kbd> |
| `search_mode` | `"smart"` | `"smart"` (regex when the query starts with `/`), `"plain"` or `"regex"` |
| `nerd_font` | `true` | Nerd Font icons; `false` uses plain Unicode symbols |
| `mouse` | `true` | Click, double-click and scroll; `false` keeps your terminal's text selection |

## `[bluetooth]`

| Key | Default | What it does |
| --- | --- | --- |
| `auto_trust_on_pair` | `true` | Trust a device after pairing, so it can reconnect on its own |
| `connection_timeout_secs` | `30` | Time limit for pair, trust and connect together. Range 5–120 |

## `[notifications]`

| Key | Default | What it does |
| --- | --- | --- |
| `success_duration_ms` | `3000` | How long a success toast stays up |
| `error_duration_ms` | `7000` | How long an error toast stays up |

## `[theme.palette]`

Colours are `"#RRGGBB"`. VoidLink never paints a background, so terminal transparency and blur show through.

| Key | Default | Used for |
| --- | --- | --- |
| `accent_primary` | `#00E5FF` | Headers, active borders, connected indicators |
| `accent_secondary` | `#BB86FC` | Selection highlight, focused elements |
| `accent_error` | `#FF4545` | Errors and destructive actions |
| `text_primary` | `#E8E6F0` | Body text |
| `text_dim` | `#6B6F85` | Secondary text |
| `paired` | `#FFB74D` | Paired but not connected |
| `success` | `#69F0AE` | Success and trusted |
| `scanning` | `#18FFFF` | The scanning spinner |
| `border_inactive` | `#4A4E69` | Inactive pane borders |

## `[keybindings]`

Every action can be remapped. [Keybindings](keybindings.md) lists the actions, their defaults and the key names you can use.

## Command-line flags

| Flag | What it does |
| --- | --- |
| `--no-nerd-font` | Plain Unicode symbols instead of Nerd Font icons |
| `--no-mouse` | Don't capture the mouse, keeping terminal text selection |
| `--print-default-config` | Print the default `config.toml` and exit |
| `--config-path` | Print where the config file lives and exit |
| `-h`, `--help` | Show help and exit |
| `-V`, `--version` | Show the version and exit |

The environment variable `VOIDLINK_LOG` sets the log level (for example `debug`); the log goes to `~/.local/state/voidlink/voidlink.log`.

[Back to the README](../README.md)
