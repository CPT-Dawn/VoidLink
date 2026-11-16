//! Unified event source: terminal input, Bluetooth worker events, Unix
//! signals and an animation tick, merged into one async `next()`.

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{Event as CtEvent, EventStream, KeyEvent, KeyEventKind, MouseEvent};
use futures::StreamExt;
use tokio::signal::unix::{signal, Signal, SignalKind};
use tokio::sync::mpsc;

use crate::bluetooth::types::BtEvent;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize,
    /// Animation tick — only produced while something is animating.
    Tick,
    Bluetooth(BtEvent),
    /// SIGTERM / SIGHUP / SIGINT: quit cleanly and restore the terminal.
    Terminate,
}

pub struct EventHandler {
    terminal: EventStream,
    tick: tokio::time::Interval,
    bt_rx: mpsc::Receiver<BtEvent>,
    bt_open: bool,
    sigterm: Signal,
    sighup: Signal,
    sigint: Signal,
}

impl EventHandler {
    pub fn new(bt_rx: mpsc::Receiver<BtEvent>) -> Result<Self> {
        let ms = crate::config::get().general.tick_rate_ms;
        let mut tick = tokio::time::interval(Duration::from_millis(ms));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        Ok(Self {
            terminal: EventStream::new(),
            tick,
            bt_rx,
            bt_open: true,
            sigterm: signal(SignalKind::terminate())?,
            sighup: signal(SignalKind::hangup())?,
            sigint: signal(SignalKind::interrupt())?,
        })
    }

    /// Wait for the next event. When `animating` is false no ticks are
    /// produced, so an idle VoidLink uses no CPU at all.
    pub async fn next(&mut self, animating: bool) -> Result<Event> {
        loop {
            tokio::select! {
                bt = self.bt_rx.recv(), if self.bt_open => match bt {
                    Some(evt) => return Ok(Event::Bluetooth(evt)),
                    None => self.bt_open = false,
                },
                term = self.terminal.next() => match term {
                    Some(Ok(CtEvent::Key(key))) if key.kind != KeyEventKind::Release => {
                        return Ok(Event::Key(key));
                    }
                    Some(Ok(CtEvent::Mouse(m))) => return Ok(Event::Mouse(m)),
                    Some(Ok(CtEvent::Resize(..))) => return Ok(Event::Resize),
                    Some(Ok(_)) => {}
                    Some(Err(e)) => return Err(e.into()),
                    // stdin closed: nothing more can ever arrive.
                    None => return Ok(Event::Terminate),
                },
                _ = self.tick.tick(), if animating => return Ok(Event::Tick),
                _ = self.sigterm.recv() => return Ok(Event::Terminate),
                _ = self.sighup.recv() => return Ok(Event::Terminate),
                _ = self.sigint.recv() => return Ok(Event::Terminate),
            }
        }
    }
}
