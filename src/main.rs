//! VoidLink — a fast, keyboard-first Bluetooth manager for the terminal.
//!
//! Architecture:
//! - **UI** (main task): runs the ratatui render loop and owns `App`.
//! - **BT worker** (tokio task): owns the bluer Session/Adapter, talks D-Bus.
//! - Two `mpsc` channels bridge them: `BtCommand` (UI→Worker), `BtEvent` (Worker→UI).
//!
//! The UI never touches D-Bus; the worker never touches the terminal.

mod app;
mod bluetooth;
mod config;
mod event;
mod theme;
mod tui;
mod ui;

use std::fs::File;
use std::process::ExitCode;
use std::sync::Mutex;

use color_eyre::Result;
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use app::{App, ToastKind};
use bluetooth::types::{BtCommand, BtEvent};
use event::Event;

const HELP: &str = "\
VoidLink — a keyboard-first Bluetooth manager for your terminal

USAGE:
    voidlink [OPTIONS]

OPTIONS:
    --no-nerd-font          Use plain Unicode symbols instead of Nerd Font icons
    --no-mouse              Don't capture the mouse (keeps terminal text selection)
    --print-default-config  Print the default config.toml and exit
    --config-path           Print where the config file lives and exit
    -h, --help              Show this help and exit
    -V, --version           Show the version and exit

Press ? inside VoidLink to see every keybinding.

FILES:
    ~/.config/voidlink/config.toml      Settings, colours and keybindings
    ~/.local/state/voidlink/voidlink.log  Log (level via VOIDLINK_LOG, e.g. debug)

REQUIRES:
    bluez with bluetooth.service running:  sudo systemctl enable --now bluetooth
";

#[derive(Debug, Default)]
struct Cli {
    no_nerd_font: bool,
    no_mouse: bool,
}

fn parse_args() -> Result<Option<Cli>, String> {
    let mut cli = Cli::default();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("voidlink {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--print-default-config" => {
                print!("{}", config::default_config_str());
                return Ok(None);
            }
            "--config-path" => {
                println!("{}", config::config_path().display());
                return Ok(None);
            }
            "--no-nerd-font" => cli.no_nerd_font = true,
            "--no-mouse" => cli.no_mouse = true,
            other => return Err(format!("unknown option '{other}'. Try 'voidlink --help'")),
        }
    }
    Ok(Some(cli))
}

/// Log to `$XDG_STATE_HOME/voidlink/voidlink.log`. Never to the terminal:
/// anything written to stdout/stderr while the TUI runs corrupts the screen.
fn init_logging() {
    let Some(dir) = directories::ProjectDirs::from("", "", "voidlink")
        .and_then(|d| d.state_dir().map(std::path::Path::to_path_buf))
    else {
        return;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let Ok(file) = File::create(dir.join("voidlink.log")) else {
        return;
    };
    let filter = EnvFilter::try_from_env("VOIDLINK_LOG")
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(false)
        .with_writer(Mutex::new(file))
        .try_init();
}

fn main() -> ExitCode {
    let cli = match parse_args() {
        Ok(Some(cli)) => cli,
        Ok(None) => return ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("voidlink: {msg}");
            return ExitCode::from(2);
        }
    };

    if let Err(e) = color_eyre::install() {
        eprintln!("voidlink: {e}");
    }
    init_logging();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("voidlink: failed to start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(run(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e:?}");
            eprintln!("voidlink: {e:?}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<()> {
    info!("VoidLink {} starting", env!("CARGO_PKG_VERSION"));
    let warnings = config::init();
    let general = &config::get().general;
    theme::use_plain_glyphs(cli.no_nerd_font || !general.nerd_font);

    let (cmd_tx, cmd_rx) = mpsc::channel::<BtCommand>(64);
    let (evt_tx, evt_rx) = mpsc::channel::<BtEvent>(256);
    let worker = spawn_worker(cmd_rx, evt_tx);

    let mut events = event::EventHandler::new(evt_rx)?;
    let mut guard = tui::init(general.mouse && !cli.no_mouse)?;
    let mut app = App::new();
    for w in warnings {
        app.toast(ToastKind::Error, w);
    }

    let mut worker_crashed = false;
    while app.running {
        if app.dirty {
            guard.terminal.draw(|frame| ui::render(frame, &mut app))?;
            app.dirty = false;
        }

        let command = match events.next(app.is_animating()).await? {
            Event::Key(key) => app.handle_key(key),
            Event::Mouse(m) => app.handle_mouse(m),
            Event::Tick => {
                app.on_tick();
                None
            }
            Event::Bluetooth(evt) => {
                app.handle_bt_event(evt);
                None
            }
            Event::Resize => {
                app.dirty = true;
                None
            }
            Event::Terminate => {
                app.running = false;
                None
            }
        };

        if let Some(cmd) = command {
            if cmd_tx.try_send(cmd).is_err() {
                app.toast(ToastKind::Error, "VoidLink is busy. Try again in a moment");
            }
        }

        if !worker_crashed && worker.is_finished() {
            worker_crashed = true;
            error!("Bluetooth worker exited unexpectedly");
            app.toast(
                ToastKind::Error,
                "The Bluetooth worker stopped unexpectedly. Please restart VoidLink",
            );
        }
    }

    // Closing the command channel makes the worker drop its discovery
    // session (stopping the scan) and exit.
    drop(cmd_tx);
    let _ = tokio::time::timeout(std::time::Duration::from_millis(500), worker).await;
    drop(guard);
    info!("VoidLink exiting");
    Ok(())
}

#[cfg(not(feature = "demo"))]
fn spawn_worker(
    cmd_rx: mpsc::Receiver<BtCommand>,
    evt_tx: mpsc::Sender<BtEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(bluetooth::worker::run(cmd_rx, evt_tx))
}

/// Demo builds (`--features demo`) run a scripted fake adapter when
/// `VOIDLINK_DEMO=1`, for screenshots and recordings without real hardware.
#[cfg(feature = "demo")]
fn spawn_worker(
    cmd_rx: mpsc::Receiver<BtCommand>,
    evt_tx: mpsc::Sender<BtEvent>,
) -> tokio::task::JoinHandle<()> {
    if std::env::var_os("VOIDLINK_DEMO").is_some() {
        tokio::spawn(bluetooth::demo::run(cmd_rx, evt_tx))
    } else {
        tokio::spawn(bluetooth::worker::run(cmd_rx, evt_tx))
    }
}
