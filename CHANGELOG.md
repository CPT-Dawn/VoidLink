# Changelog

All notable changes to VoidLink are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [1.1.0] - 2026-11-16

A production-hardening release: safer pairing, a far more reliable
Bluetooth backend, and a redesigned, friendlier interface.

### Security

- **Pairing now asks you.** Previously, while VoidLink was running, every
  pairing confirmation and service-authorization request was accepted
  automatically, and PIN/passkey requests were answered with `0000` / `0`.
  Every request is now a prompt: confirm the code, allow/deny, or type the
  PIN. Unanswered prompts are rejected after 24 seconds.

### Added

- Prebuilt binaries for **x86_64** and **aarch64** on every release, and a new
  [`voidlink-bin`](https://aur.archlinux.org/packages/voidlink-bin) AUR package.
- Live updates: connection state, battery and signal now refresh as they
  change, including changes made by other tools or by the device itself.
- Automatic recovery when `bluetoothd` restarts or the adapter is unplugged,
  with an explanation of what to do while it's unavailable.
- Clear, actionable error messages (rfkill blocks, a missing audio stack,
  devices out of range, a stopped service…).
- Device list grouped into **My devices** and **Nearby**, with friendly
  labels for unnamed beacons.
- Confirmation before forgetting a device.
- Mouse support: click to select, double-click to connect, scroll to move.
- Responsive layout down to 44×12, with stacked panels on narrow terminals
  and a two-column help screen on short ones.
- Plain-Unicode mode for terminals without a Nerd Font (`--no-nerd-font` or
  `general.nerd_font = false`).
- Command-line flags: `--help`, `--version`, `--no-nerd-font`, `--no-mouse`,
  `--print-default-config`, `--config-path`.
- Config problems (syntax errors, unknown keys or colours, conflicting
  keybindings) are reported in the app instead of being silently ignored.
- Logs are written to `~/.local/state/voidlink/voidlink.log`
  (`VOIDLINK_LOG=debug` for more detail).

### Changed

- Connecting, pairing and other device operations run concurrently, so a
  slow device never freezes scanning or the rest of the app.
- Notifications are non-blocking toasts instead of modal popups.
- Key hints and the help screen show your remapped keys.
- Renaming a device now shows the new name everywhere.
- VoidLink uses no CPU while idle (it previously redrew 60 times a second).
- Default `tick_rate_ms` is now 33; `notifications.slide_speed` was removed
  (old configs still load).
- Minimum supported Rust version is now 1.88.

### Fixed

- The cursor could jump to a different device when the list re-sorted, so
  Enter could act on the wrong device. Selection now follows the device.
- Spinners stuck forever after a failed disconnect or removal.
- "Worker crashed" errors repeating on every event when no adapter existed.
- Log output corrupting the screen.
- The terminal not being restored after an error or `SIGTERM`/`SIGHUP`.
- An active search filter being invisible after pressing Enter.
- An invalid regex hiding every device.
- An unwritable config directory aborting startup.

## [1.0.0] - 2026-03-22

- Initial release.

[1.1.0]: https://github.com/CPT-Dawn/VoidLink/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/CPT-Dawn/VoidLink/releases/tag/v1.0.0
