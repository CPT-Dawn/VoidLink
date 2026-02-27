//! Unified event loop that merges crossterm terminal events, Bluetooth worker
//! events, and a fixed-rate tick into a single async stream.
//!
//! The TUI main loop `select!`s over `EventHandler::next()` to process all
//! three sources without blocking the render path.

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, KeyEventKind};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::bluetooth::types::BtEvent;

/// Unified event type consumed by the TUI main loop.
#[derive(Debug)]
pub enum Event {
    /// A key was pressed (only `Press` kind — ignores release/repeat on
    /// platforms that emit them).
    Key(KeyEvent),
    /// Terminal was resized.
    #[allow(dead_code)]
    Resize(u16, u16),
    /// Animation / state tick.
    Tick,
    /// An event from the Bluetooth worker task.
    Bluetooth(BtEvent),
}

/// Multiplexes crossterm events, a tick timer, and the Bluetooth event channel
/// into a single `Event` stream.
pub struct EventHandler {
    /// Async crossterm event reader.
    crossterm_stream: EventStream,
    /// Tick interval for animations.
    tick_interval: tokio::time::Interval,
    /// Receiver end of the BT worker → UI channel.
    bt_rx: mpsc::Receiver<BtEvent>,
}

impl EventHandler {
    pub fn new(bt_rx: mpsc::Receiver<BtEvent>) -> Self {
        let tick_ms = crate::config::get().general.tick_rate_ms;
        let mut tick_interval = tokio::time::interval(Duration::from_millis(tick_ms));
        // Don't try to "catch up" missed ticks — just keep going.
        tick_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        Self {
            crossterm_stream: EventStream::new(),
            tick_interval,
            bt_rx,
        }
    }

    /// Await the next event from any source. Returns `None` only when all
    /// sources are exhausted (which shouldn't happen during normal operation).
    pub async fn next(&mut self) -> Result<Event> {
        loop {
            tokio::select! {
                // ── Bluetooth events (highest priority) ─────────────────
                Some(bt_event) = self.bt_rx.recv() => {
                    return Ok(Event::Bluetooth(bt_event));
                }

                // ── Terminal events ─────────────────────────────────────
                Some(ct_result) = self.crossterm_stream.next() => {
                    match ct_result? {
                        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                            return Ok(Event::Key(key));
                        }
                        CrosstermEvent::Resize(w, h) => return Ok(Event::Resize(w, h)),
                        // Swallow key release/repeat — loop again instead
                        // of emitting a Tick that would trigger a redraw.
                        _ => continue,
                    }
                }

                // ── Tick timer ──────────────────────────────────────────
                _ = self.tick_interval.tick() => {
                    return Ok(Event::Tick);
                }
            }
        }
    }
}
