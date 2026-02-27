//! Application state and input handling.
//!
//! `App` is the single source of truth for the entire TUI. It is only mutated
//! from the main event loop — no `Arc<Mutex<>>` needed.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tokio::sync::mpsc;

use crate::bluetooth::types::*;

// ─── Input modes ────────────────────────────────────────────────────────────

/// Which mode the UI is currently in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// Normal vim-style navigation.
    Normal,
    /// `/` search — keys go to the search buffer.
    Search,
    /// A popup dialog is displayed.
    Dialog,
}

// ─── Popup types ────────────────────────────────────────────────────────────

/// Active popup overlay.
#[derive(Debug, Clone)]
pub enum Popup {
    /// An error message with a sliding animation progress (0.0 → 1.0).
    Error { message: String, slide: f32 },
    /// Connection result notification.
    ConnectionResult {
        #[allow(dead_code)]
        address: Address,
        success: bool,
        message: String,
        slide: f32,
    },
    /// PIN display during pairing.
    PinDisplay {
        #[allow(dead_code)]
        address: Address,
        pin: String,
        slide: f32,
    },
    /// Help overlay.
    Help,
}

impl Popup {
    /// Get mutable reference to the slide progress, if applicable.
    pub fn slide_mut(&mut self) -> Option<&mut f32> {
        match self {
            Popup::Error { slide, .. }
            | Popup::ConnectionResult { slide, .. }
            | Popup::PinDisplay { slide, .. } => Some(slide),
            Popup::Help => None,
        }
    }

    /// Get the current slide progress.
    #[allow(dead_code)]
    pub fn slide(&self) -> f32 {
        match self {
            Popup::Error { slide, .. }
            | Popup::ConnectionResult { slide, .. }
            | Popup::PinDisplay { slide, .. } => *slide,
            Popup::Help => 1.0,
        }
    }
}

// ─── Actions produced by input handling ─────────────────────────────────────

/// Actions that the main loop should execute after processing input.
#[derive(Debug)]
pub enum AppAction {
    /// Quit the application.
    Quit,
    /// Send a command to the Bluetooth worker.
    BtCommand(BtCommand),
    /// No-op (event was consumed but requires no further action).
    Consumed,
}

// ─── App state ──────────────────────────────────────────────────────────────

pub struct App {
    /// All known devices, sorted by `DeviceInfo::sort_key()`.
    pub devices: Vec<DeviceInfo>,
    /// Index of the selected device in the (possibly filtered) list.
    pub selected_index: usize,
    /// Current adapter snapshot.
    pub adapter: AdapterInfo,
    /// Whether scanning is active.
    pub scanning: bool,
    /// Current input mode.
    pub input_mode: InputMode,
    /// Active search query (when in Search mode).
    pub search_query: String,
    /// Active popup overlay.
    pub active_popup: Option<Popup>,
    /// Monotonic tick counter for animations.
    pub tick_count: u64,
    /// Auto-dismiss countdown for transient popups (in ticks).
    pub popup_ttl: Option<u64>,
    /// Whether the application should keep running.
    pub running: bool,
    /// Sender handle to the BT worker (kept for future use in async actions).
    #[allow(dead_code)]
    pub bt_cmd_tx: mpsc::Sender<BtCommand>,
}

impl App {
    pub fn new(bt_cmd_tx: mpsc::Sender<BtCommand>) -> Self {
        Self {
            devices: Vec::new(),
            selected_index: 0,
            adapter: AdapterInfo::default(),
            scanning: false,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            active_popup: None,
            tick_count: 0,
            popup_ttl: None,
            running: true,
            bt_cmd_tx,
        }
    }

    // ── Filtered device list ────────────────────────────────────────────

    /// Return the device list filtered by the current search query
    /// and the `hide_unnamed_devices` config setting.
    pub fn filtered_devices(&self) -> Vec<&DeviceInfo> {
        let hide_unnamed = crate::config::get().general.hide_unnamed_devices;

        self.devices
            .iter()
            .filter(|d| {
                // Optionally hide unnamed (address-only) devices.
                if hide_unnamed && d.name.is_none() {
                    return false;
                }
                // Apply search filter.
                if self.search_query.is_empty() {
                    return true;
                }
                let query = self.search_query.to_lowercase();
                d.display_name().to_lowercase().contains(&query)
                    || d.address.to_string().to_lowercase().contains(&query)
            })
            .collect()
    }

    /// The currently selected device (if any).
    pub fn selected_device(&self) -> Option<&DeviceInfo> {
        let filtered = self.filtered_devices();
        filtered.get(self.selected_index).copied()
    }

    /// Clamp `selected_index` to valid bounds.
    fn clamp_selection(&mut self) {
        let len = self.filtered_devices().len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    // ── Tick handling ───────────────────────────────────────────────────

    /// Called on every animation tick (~60 Hz).
    pub fn on_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);

        // Animate popup slide-in.
        if let Some(popup) = &mut self.active_popup {
            if let Some(slide) = popup.slide_mut() {
                if *slide < 1.0 {
                    let speed = crate::config::get().notifications.slide_speed;
                    *slide = (*slide + speed).min(1.0);
                }
            }
        }

        // Auto-dismiss transient popups.
        if let Some(ttl) = &mut self.popup_ttl {
            if *ttl == 0 {
                self.active_popup = None;
                self.popup_ttl = None;
                if self.input_mode == InputMode::Dialog {
                    self.input_mode = InputMode::Normal;
                }
            } else {
                *ttl -= 1;
            }
        }
    }

    // ── Bluetooth event handling ────────────────────────────────────────

    /// Apply a Bluetooth event from the worker to the app state.
    pub fn handle_bt_event(&mut self, event: BtEvent) {
        match event {
            BtEvent::AdapterState(info) => {
                self.adapter = info;
            }

            BtEvent::DeviceFound(info) => {
                // Insert or update.
                if let Some(existing) = self.devices.iter_mut().find(|d| d.address == info.address)
                {
                    *existing = info;
                } else {
                    self.devices.push(info);
                }
                self.sort_devices();
                self.clamp_selection();
            }

            BtEvent::DeviceUpdated(info) => {
                if let Some(existing) = self.devices.iter_mut().find(|d| d.address == info.address)
                {
                    *existing = info;
                } else {
                    self.devices.push(info);
                }
                self.sort_devices();
                self.clamp_selection();
            }

            BtEvent::DeviceRemoved(addr) => {
                self.devices.retain(|d| d.address != addr);
                self.clamp_selection();
            }

            BtEvent::ConnectionResult {
                address,
                success,
                error,
            } => {
                let message = if success {
                    format!("Connected to {address}")
                } else {
                    format!(
                        "Connection failed: {}",
                        error.as_deref().unwrap_or("unknown error")
                    )
                };
                self.show_transient_popup(Popup::ConnectionResult {
                    address,
                    success,
                    message,
                    slide: 0.0,
                });
            }

            BtEvent::PairResult {
                address: _,
                success,
                error,
            } => {
                if !success {
                    let message = format!(
                        "Pairing failed: {}",
                        error.as_deref().unwrap_or("unknown error")
                    );
                    self.show_transient_popup(Popup::Error {
                        message,
                        slide: 0.0,
                    });
                }
            }

            BtEvent::PinRequest { address, pin } => {
                self.active_popup = Some(Popup::PinDisplay {
                    address,
                    pin,
                    slide: 0.0,
                });
                self.input_mode = InputMode::Dialog;
                // PIN dialogs stay until dismissed.
                self.popup_ttl = None;
            }

            BtEvent::ScanningChanged(scanning) => {
                self.scanning = scanning;
            }

            BtEvent::Error(msg) => {
                self.show_transient_popup(Popup::Error {
                    message: msg,
                    slide: 0.0,
                });
            }
        }
    }

    /// Show a transient popup with timeout tuned to message severity.
    fn show_transient_popup(&mut self, popup: Popup) {
        let notif = &crate::config::get().notifications;
        let tick_ms = crate::config::get().general.tick_rate_ms.max(1);

        let duration_ms = match &popup {
            Popup::ConnectionResult { success: true, .. } => notif.success_duration_ms,
            Popup::ConnectionResult { success: false, .. } | Popup::Error { .. } => {
                notif.error_duration_ms
            }
            Popup::PinDisplay { .. } | Popup::Help => notif.success_duration_ms,
        };

        self.active_popup = Some(popup);
        self.input_mode = InputMode::Dialog;
        self.popup_ttl = Some(duration_ms / tick_ms);
    }

    /// Re-sort devices by sort key (connected first, then RSSI descending).
    fn sort_devices(&mut self) {
        self.devices.sort_by_key(|a| a.sort_key());
    }

    // ── Input handling ──────────────────────────────────────────────────

    /// Process a key event and return an action for the main loop.
    pub fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::Search => self.handle_search_key(key),
            InputMode::Dialog => self.handle_dialog_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> AppAction {
        let kb = &crate::config::get().keys;

        // Ctrl+C always quits (system convention, non-configurable).
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return AppAction::Quit;
        }

        match key.code {
            // ── Quit ────────────────────────────────────────────────────
            c if c == kb.quit => AppAction::Quit,

            // ── Navigation ──────────────────────────────────────────────
            c if c == kb.nav_down || c == KeyCode::Down => {
                let len = self.filtered_devices().len();
                if len > 0 {
                    self.selected_index = (self.selected_index + 1).min(len - 1);
                }
                AppAction::Consumed
            }
            c if c == kb.nav_up || c == KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
                AppAction::Consumed
            }
            c if c == kb.jump_top => {
                self.selected_index = 0;
                AppAction::Consumed
            }
            c if c == kb.jump_bottom => {
                let len = self.filtered_devices().len();
                if len > 0 {
                    self.selected_index = len - 1;
                }
                AppAction::Consumed
            }

            // ── Search ──────────────────────────────────────────────────
            c if c == kb.search => {
                self.input_mode = InputMode::Search;
                self.search_query.clear();
                AppAction::Consumed
            }

            // ── Help ────────────────────────────────────────────────────
            c if c == kb.help => {
                self.active_popup = Some(Popup::Help);
                self.input_mode = InputMode::Dialog;
                self.popup_ttl = None;
                AppAction::Consumed
            }

            // ── Adapter controls ────────────────────────────────────────
            c if c == kb.toggle_adapter => {
                if self.adapter.powered {
                    AppAction::BtCommand(BtCommand::DisableAdapter)
                } else {
                    AppAction::BtCommand(BtCommand::EnableAdapter)
                }
            }
            c if c == kb.toggle_scan => {
                if self.scanning {
                    AppAction::BtCommand(BtCommand::StopScan)
                } else {
                    AppAction::BtCommand(BtCommand::StartScan)
                }
            }

            // ── Device actions ──────────────────────────────────────────
            c if c == kb.connect_toggle => {
                if let Some(device) = self.selected_device() {
                    let addr = device.address;
                    if device.connected {
                        AppAction::BtCommand(BtCommand::Disconnect(addr))
                    } else {
                        AppAction::BtCommand(BtCommand::Connect(addr))
                    }
                } else {
                    AppAction::Consumed
                }
            }
            c if c == kb.disconnect => {
                if let Some(device) = self.selected_device() {
                    let addr = device.address;
                    AppAction::BtCommand(BtCommand::Disconnect(addr))
                } else {
                    AppAction::Consumed
                }
            }
            c if c == kb.pair => {
                if let Some(device) = self.selected_device() {
                    let addr = device.address;
                    AppAction::BtCommand(BtCommand::Pair(addr))
                } else {
                    AppAction::Consumed
                }
            }
            c if c == kb.trust => {
                if let Some(device) = self.selected_device() {
                    let addr = device.address;
                    AppAction::BtCommand(BtCommand::Trust(addr))
                } else {
                    AppAction::Consumed
                }
            }
            c if c == kb.remove => {
                if let Some(device) = self.selected_device() {
                    let addr = device.address;
                    AppAction::BtCommand(BtCommand::RemoveDevice(addr))
                } else {
                    AppAction::Consumed
                }
            }
            c if c == kb.refresh => {
                if let Some(device) = self.selected_device() {
                    let addr = device.address;
                    AppAction::BtCommand(BtCommand::RefreshDevice(addr))
                } else {
                    AppAction::Consumed
                }
            }

            _ => AppAction::Consumed,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.search_query.clear();
                self.clamp_selection();
                AppAction::Consumed
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                self.clamp_selection();
                AppAction::Consumed
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.selected_index = 0;
                AppAction::Consumed
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.selected_index = 0;
                AppAction::Consumed
            }
            _ => AppAction::Consumed,
        }
    }

    fn handle_dialog_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {
                self.active_popup = None;
                self.popup_ttl = None;
                self.input_mode = InputMode::Normal;
                AppAction::Consumed
            }
            _ => AppAction::Consumed,
        }
    }
}
