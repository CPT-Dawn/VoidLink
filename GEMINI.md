# VoidLink Context

VoidLink is a memory-safe, keyboard-first Bluetooth manager for Linux terminals, engineered for modern Wayland workflows. It provides a terminal user interface (TUI) for interacting with the BlueZ Bluetooth stack via D-Bus.

## Architecture

The application follows a strict separation of concerns between the user interface and system interactions:

- **UI Thread (Main):**
  - Uses `ratatui` for rendering and `crossterm` for terminal handling.
  - Owns the `App` state, which is the single source of truth for the TUI.
  - Processes keyboard, resize, and tick events.
  - Communicates with the Bluetooth worker via `BtCommand` (mpsc channel).
  - Employs a dirty-flag optimization to redraw only when state changes.

- **Bluetooth Worker (Tokio Task):**
  - Owns the `bluer::Session` and `Adapter`.
  - Registers a custom BlueZ Agent for passkey/PIN forwarding.
  - Executes BlueZ operations (scanning, pairing, connecting) over D-Bus.
  - Emits `BtEvent` snapshots/results back to the UI thread via a separate mpsc channel.

## Tech Stack

- **Language:** Rust (1.75+)
- **TUI:** `ratatui`, `crossterm`
- **Bluetooth:** `bluer` (BlueZ interface)
- **Async Runtime:** `tokio`
- **Configuration:** `serde`, `toml`
- **Error Handling:** `color-eyre`

## Building and Running

### Development
- **Build:** `cargo build --release`
- **Run:** `cargo run --release`
- **Test:** `cargo test` (Note: Most Bluetooth logic requires a running BlueZ daemon)

### Installation (Arch Linux)
- `paru -S voidlink` or `yay -S voidlink`

## Development Conventions

### State Management
- All UI-related state resides in `src/app.rs` within the `App` struct.
- Mutation of `App` state happens exclusively in the main event loop in `src/main.rs`.
- Avoid using `Arc<Mutex<>>` for UI state; rely on the channel-based communication with the worker.

### Bluetooth Operations
- Define new commands in `src/bluetooth/types.rs` under `BtCommand`.
- Define new event responses in `src/bluetooth/types.rs` under `BtEvent`.
- Implement the worker-side logic in `src/bluetooth/worker.rs`.
- Bluetooth properties are snapshotted into plain data structures (`DeviceInfo`, `AdapterInfo`) before being sent to the UI thread to avoid leaking D-Bus handles.

### UI Rendering
- UI components are modularized in `src/ui/`.
- The `render` function in `src/ui/mod.rs` dispatches to sub-modules (`device_list`, `detail_panel`, etc.).
- The theme is centralized in `src/theme.rs`.

### Configuration
- Default configuration is embedded in the binary via `default_config.toml`.
- User configuration is stored in `~/.config/voidlink/config.toml`.
- Handled in `src/config.rs`.
