//! Terminal lifecycle management.
//!
//! Handles entering/leaving the alternate screen, enabling/disabling raw mode,
//! and installing a panic hook that restores the terminal before printing the
//! backtrace. This prevents leaving the user's shell in a broken state.

use std::io::{stdout, Stdout};

use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

/// Convenience alias.
pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Enter the alternate screen, enable raw mode, and install the panic hook.
pub fn init() -> Result<Tui> {
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    install_panic_hook();
    let backend = CrosstermBackend::new(stdout());
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Leave the alternate screen and disable raw mode.
pub fn restore() -> Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Install a panic hook that restores the terminal *before* printing the
/// default panic message. Without this, a panic leaves raw mode active and
/// the alternate screen visible, making the error unreadable.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Best-effort restore — ignore errors since we're already panicking.
        let _ = restore();
        original_hook(panic_info);
    }));
}
