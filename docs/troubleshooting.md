# Troubleshooting

The messages VoidLink shows most often, each with its fix. The messages come from [`src/bluetooth/errors.rs`](../src/bluetooth/errors.rs).

| VoidLink says | Fix |
| --- | --- |
| The Bluetooth service (bluetoothd) isn't running | Install BlueZ and run `sudo systemctl enable --now bluetooth.service`. VoidLink reconnects once the service is up |
| Bluetooth is blocked by rfkill | Run `rfkill unblock bluetooth`; check with `rfkill list`. A laptop's wireless switch can also block it |
| No Bluetooth adapter found | Plug one in, or check `rfkill list bluetooth` |
| No audio profile is available | Your audio server lacks Bluetooth support. Install `pipewire-audio` (or `pulseaudio-bluetooth`), then run `systemctl --user restart pipewire wireplumber` or log in again |
| The device didn't respond | Make sure it's on, nearby and in pairing mode |
| Pairing was rejected | The codes didn't match or the device refused. Check the code and try again |
| Permission denied by BlueZ | Your D-Bus or polkit policy blocks your user; check your distro's Bluetooth group or polkit rules |
| Can't reach the system D-Bus | Start D-Bus; on systemd distributions it runs as `dbus.service` |

## Boxes or question marks instead of icons

Your terminal font has no Nerd Font glyphs. Install one (for example `ttf-nerd-fonts-symbols-mono`), or run `voidlink --no-nerd-font`.

## Text selection doesn't work while VoidLink is open

VoidLink captures the mouse. Most terminals let you select anyway while you hold <kbd>Shift</kbd>. To turn mouse capture off, run `voidlink --no-mouse` or set `mouse = false`.

## Something else

1. Run `VOIDLINK_LOG=debug voidlink` and reproduce the problem.
2. Open an [issue](https://github.com/CPT-Dawn/VoidLink/issues/new/choose) and attach `~/.local/state/voidlink/voidlink.log`.

[Back to the README](../README.md)
