//! VoidLink — A blazing-fast, memory-safe Bluetooth TUI manager for Linux.
//!
//! Architecture:
//! - **UI thread** (main): runs the ratatui render loop, processes key events.
//! - **BT worker** (tokio task): owns the bluer Session/Adapter, talks D-Bus.
//! - Two `mpsc` channels bridge them: `BtCommand` (UI→Worker), `BtEvent` (Worker→UI).
//!
//! The UI thread never touches D-Bus. The worker thread never touches the terminal.

mod app;
mod bluetooth;
mod event;
mod theme;
mod tui;
mod ui;

use color_eyre::Result;
use tokio::sync::mpsc;
use tracing::info;
use tracing_subscriber::EnvFilter;

use app::{App, AppAction};
use bluetooth::types::BtCommand;
use event::Event;

#[tokio::main]
async fn main() -> Result<()> {
    // ── Error handling & logging ─────────────────────────────────────────
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .with_target(false)
        .with_writer(std::io::stderr) // TUI owns stdout; logs go to stderr
        .init();

    info!("VoidLink starting");

    // ── Channel setup ───────────────────────────────────────────────────
    let (bt_cmd_tx, bt_cmd_rx) = mpsc::channel::<BtCommand>(32);
    let (bt_evt_tx, bt_evt_rx) = mpsc::channel(64);

    // ── Spawn Bluetooth worker ──────────────────────────────────────────
    tokio::spawn(async move {
        bluetooth::worker::run(bt_cmd_rx, bt_evt_tx).await;
    });

    // ── Initialise terminal ─────────────────────────────────────────────
    let mut terminal = tui::init()?;

    // ── App state ───────────────────────────────────────────────────────
    let mut app = App::new(bt_cmd_tx.clone());
    let mut events = event::EventHandler::new(bt_evt_rx);

    // ── Main event loop ─────────────────────────────────────────────────
    while app.running {
        // Render.
        terminal.draw(|frame| ui::render(frame, &app))?;

        // Await next event (key / tick / BT).
        match events.next().await? {
            Event::Key(key) => {
                let action = app.handle_key(key);
                match action {
                    AppAction::Quit => {
                        app.running = false;
                    }
                    AppAction::BtCommand(cmd) => {
                        // Non-blocking send; drop if worker is backed up.
                        let _ = bt_cmd_tx.try_send(cmd);
                    }
                    AppAction::Consumed => {}
                }
            }
            Event::Tick => {
                app.on_tick();
            }
            Event::Bluetooth(bt_event) => {
                app.handle_bt_event(bt_event);
            }
            Event::Resize(_, _) => {
                // ratatui handles resize automatically on next draw.
            }
        }
    }

    // ── Cleanup ─────────────────────────────────────────────────────────
    tui::restore()?;
    info!("VoidLink exiting");
    Ok(())
}
