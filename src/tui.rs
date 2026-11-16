//! Terminal lifecycle: raw mode, alternate screen, mouse capture, and a
//! guard + panic hook that always restore the user's terminal.

use std::io::{stdout, Stdout};
use std::sync::atomic::{AtomicBool, Ordering};

use color_eyre::Result;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

static ACTIVE: AtomicBool = AtomicBool::new(false);
static MOUSE: AtomicBool = AtomicBool::new(false);

/// Restores the terminal when dropped — on normal exit *and* on `?` errors.
pub struct TerminalGuard {
    pub terminal: Tui,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
        let _ = self.terminal.show_cursor();
    }
}

/// Enter the alternate screen and raw mode, and install the panic hook.
pub fn init(mouse: bool) -> Result<TerminalGuard> {
    install_panic_hook();
    enable_raw_mode()?;
    ACTIVE.store(true, Ordering::SeqCst);
    execute!(stdout(), EnterAlternateScreen)?;
    if mouse {
        execute!(stdout(), EnableMouseCapture)?;
        MOUSE.store(true, Ordering::SeqCst);
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.clear()?;
    Ok(TerminalGuard { terminal })
}

/// Undo `init`. Safe to call more than once.
pub fn restore() {
    if !ACTIVE.swap(false, Ordering::SeqCst) {
        return;
    }
    if MOUSE.swap(false, Ordering::SeqCst) {
        let _ = execute!(stdout(), DisableMouseCapture);
    }
    let _ = execute!(stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// Restore the terminal *before* the panic message prints, so it's readable.
fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        original(info);
    }));
}
