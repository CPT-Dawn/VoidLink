//! Application state and input handling.
//!
//! `App` is the single source of truth for the TUI. It is only mutated from
//! the main event loop — no `Arc<Mutex<>>` needed. Input handlers return the
//! `BtCommand` (if any) the main loop should forward to the worker.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::widgets::ListState;
use regex::Regex;
use tokio::sync::oneshot;

use crate::bluetooth::agent;
use crate::bluetooth::types::{
    AdapterInfo, Address, AgentReply, AgentRequest, BtCommand, BtEvent, DeviceInfo, Operation,
    PromptKind,
};
use crate::config::{self, SearchMode, SortMode};

/// How many toasts can be stacked at once.
const MAX_TOASTS: usize = 3;
/// Two clicks on the same row within this window count as a double-click.
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
/// Rows moved by PageUp / PageDown.
const PAGE: isize = 8;

// ─── State types ────────────────────────────────────────────────────────────

/// Where keystrokes go when no modal is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Search,
    Rename,
}

/// Connection state of the Bluetooth backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BtStatus {
    Starting,
    Ready,
    Unavailable(String),
}

/// A pairing question from BlueZ that the user must answer.
#[derive(Debug)]
pub struct Prompt {
    pub address: Address,
    pub kind: PromptKind,
    reply: Option<oneshot::Sender<AgentReply>>,
    pub input: String,
    pub error: Option<String>,
    pub opened: Instant,
}

impl Prompt {
    pub fn needs_input(&self) -> bool {
        matches!(self.kind, PromptKind::EnterPin | PromptKind::EnterPasskey)
    }

    /// Seconds left before BlueZ gives up on this request.
    pub fn seconds_left(&self) -> u64 {
        agent::PROMPT_TIMEOUT
            .saturating_sub(self.opened.elapsed())
            .as_secs()
    }
}

/// A focused overlay that captures input.
#[derive(Debug)]
pub enum Modal {
    Help,
    Prompt(Prompt),
    /// Show a code the user must type on the device.
    PairingCode {
        address: Address,
        code: String,
    },
    /// "Forget this device?" confirmation.
    ConfirmRemove(Address),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

/// A non-blocking notification in the corner.
#[derive(Debug, Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub message: String,
    pub shown: Instant,
    ttl: Duration,
}

impl Toast {
    /// Entrance animation progress, 0 → 1 over 180 ms.
    pub fn progress(&self) -> f32 {
        (self.shown.elapsed().as_secs_f32() / 0.18).min(1.0)
    }
}

/// One line of the device list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    /// Section title with the number of devices in it.
    Header(Section, usize),
    /// Index into `App::devices`.
    Device(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Mine,
    Nearby,
}

impl Section {
    pub fn title(self) -> &'static str {
        match self {
            Self::Mine => "MY DEVICES",
            Self::Nearby => "NEARBY",
        }
    }
}

// ─── App ────────────────────────────────────────────────────────────────────

pub struct App {
    /// All known devices, kept sorted by the active sort mode.
    pub devices: Vec<DeviceInfo>,
    /// What the list shows: filtered, sorted, grouped.
    pub rows: Vec<Row>,
    /// The selected device. Selection follows the device, not the row, so
    /// re-sorting never moves the cursor onto a different device.
    pub selected: Option<Address>,
    pub adapter: AdapterInfo,
    pub status: BtStatus,
    pub scanning: bool,
    pub mode: Mode,
    pub modal: Option<Modal>,
    pub toasts: VecDeque<Toast>,
    pub search_query: String,
    pub search_error: Option<String>,
    search_regex: Option<Regex>,
    pub rename_buffer: String,
    rename_target: Option<Address>,
    /// In-flight operation per device (drives the spinners).
    pub pending: HashMap<Address, Operation>,
    pub sort_mode: SortMode,
    /// Scroll state of the device list; the renderer updates its offset.
    pub list_state: ListState,
    /// Inner area of the device list from the last frame (for mouse hits).
    pub list_area: Rect,
    last_click: Option<(Instant, Address)>,
    started: Instant,
    pub running: bool,
    /// Whether the UI needs a redraw.
    pub dirty: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            rows: Vec::new(),
            selected: None,
            adapter: AdapterInfo::default(),
            status: BtStatus::Starting,
            scanning: false,
            mode: Mode::Normal,
            modal: None,
            toasts: VecDeque::new(),
            search_query: String::new(),
            search_error: None,
            search_regex: None,
            rename_buffer: String::new(),
            rename_target: None,
            pending: HashMap::new(),
            sort_mode: config::get().general.sort_mode,
            list_state: ListState::default(),
            list_area: Rect::default(),
            last_click: None,
            started: Instant::now(),
            running: true,
            dirty: true,
        }
    }

    /// Milliseconds since start; drives spinners independent of tick rate.
    pub fn elapsed_ms(&self) -> u128 {
        self.started.elapsed().as_millis()
    }

    /// Whether anything on screen is moving (so ticks should redraw).
    pub fn is_animating(&self) -> bool {
        self.scanning
            || !self.pending.is_empty()
            || !self.toasts.is_empty()
            || matches!(self.modal, Some(Modal::Prompt(_)))
            || self.status == BtStatus::Starting
    }

    pub fn is_ready(&self) -> bool {
        self.status == BtStatus::Ready
    }

    // ── Queries ─────────────────────────────────────────────────────────

    pub fn device(&self, addr: Address) -> Option<&DeviceInfo> {
        self.devices.iter().find(|d| d.address == addr)
    }

    pub fn selected_device(&self) -> Option<&DeviceInfo> {
        self.selected.and_then(|a| self.device(a))
    }

    /// Row index of the selected device.
    pub fn selected_row(&self) -> Option<usize> {
        let addr = self.selected?;
        self.rows.iter().position(|r| match r {
            Row::Device(i) => self.devices[*i].address == addr,
            Row::Header(..) => false,
        })
    }

    /// Number of devices currently listed.
    pub fn visible_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| matches!(r, Row::Device(_)))
            .count()
    }

    /// Name for messages: alias if known, else the address.
    pub fn name_of(&self, addr: Address) -> String {
        self.device(addr)
            .map_or_else(|| addr.to_string(), |d| d.label())
    }

    pub fn filter_active(&self) -> bool {
        !self.search_query.is_empty()
    }

    // ── List maintenance ────────────────────────────────────────────────

    fn sort_devices(&mut self) {
        match self.sort_mode {
            SortMode::Default => self.devices.sort_by_cached_key(DeviceInfo::sort_key),
            SortMode::Name => self
                .devices
                .sort_by_cached_key(|d| (d.display_name().to_lowercase(), d.address)),
            SortMode::Rssi => self
                .devices
                .sort_by_key(|d| (std::cmp::Reverse(d.rssi.unwrap_or(i16::MIN)), d.address)),
            SortMode::Address => self.devices.sort_by_key(|d| d.address),
        }
    }

    fn matches(&self, d: &DeviceInfo) -> bool {
        if config::get().general.hide_unnamed_devices && d.name.is_none() && !d.is_known() {
            return false;
        }
        if self.search_query.is_empty() {
            return true;
        }
        let addr = d.address.to_string();
        match &self.search_regex {
            Some(re) => re.is_match(d.display_name()) || re.is_match(&addr),
            None if self.regex_requested() => true, // invalid regex: don't hide everything
            None => {
                let q = self.search_query.to_lowercase();
                d.display_name().to_lowercase().contains(&q) || addr.to_lowercase().contains(&q)
            }
        }
    }

    /// Re-sort, re-filter and re-group the list, keeping the selection on the
    /// same device (or its neighbour if it disappeared).
    pub fn refresh_rows(&mut self) {
        let prev_row = self.selected_row().or(self.list_state.selected());
        self.sort_devices();

        let visible: Vec<usize> = (0..self.devices.len())
            .filter(|&i| self.matches(&self.devices[i]))
            .collect();

        self.rows.clear();
        if self.sort_mode == SortMode::Default {
            let (mine, nearby): (Vec<usize>, Vec<usize>) = visible
                .into_iter()
                .partition(|&i| self.devices[i].is_known());
            for (section, group) in [(Section::Mine, mine), (Section::Nearby, nearby)] {
                if !group.is_empty() {
                    self.rows.push(Row::Header(section, group.len()));
                    self.rows.extend(group.into_iter().map(Row::Device));
                }
            }
        } else {
            self.rows.extend(visible.into_iter().map(Row::Device));
        }

        if self.selected_row().is_none() {
            // Pick the device row closest to where the cursor was.
            let target = prev_row.unwrap_or(0);
            let device_rows = self.rows.iter().enumerate().filter_map(|(ri, r)| match r {
                Row::Device(i) => Some((ri, *i)),
                Row::Header(..) => None,
            });
            self.selected = device_rows
                .min_by_key(|(ri, _)| ri.abs_diff(target))
                .map(|(_, i)| self.devices[i].address);
        }
        self.list_state.select(self.selected_row());
        self.dirty = true;
    }

    /// Move the cursor by `delta` devices (headers are skipped).
    fn move_selection(&mut self, delta: isize) {
        let device_rows: Vec<usize> = self
            .rows
            .iter()
            .filter_map(|r| match r {
                Row::Device(i) => Some(*i),
                Row::Header(..) => None,
            })
            .collect();
        if device_rows.is_empty() {
            return;
        }
        let current = self
            .selected
            .and_then(|a| {
                device_rows
                    .iter()
                    .position(|&i| self.devices[i].address == a)
            })
            .unwrap_or(0);
        let last = device_rows.len() - 1;
        let next = current.saturating_add_signed(delta).min(last);
        self.selected = Some(self.devices[device_rows[next]].address);
        self.list_state.select(self.selected_row());
    }

    fn select_edge(&mut self, last: bool) {
        self.move_selection(if last { isize::MAX / 2 } else { isize::MIN / 2 });
    }

    // ── Search ──────────────────────────────────────────────────────────

    fn regex_requested(&self) -> bool {
        match config::get().general.search_mode {
            SearchMode::Regex => true,
            SearchMode::Smart => self.search_query.starts_with('/'),
            SearchMode::Plain => false,
        }
    }

    fn update_search(&mut self) {
        self.search_regex = None;
        self.search_error = None;
        if self.regex_requested() {
            let pattern = match config::get().general.search_mode {
                SearchMode::Smart => &self.search_query[1..],
                _ => self.search_query.as_str(),
            };
            if !pattern.is_empty() {
                match Regex::new(&format!("(?i){pattern}")) {
                    Ok(re) => self.search_regex = Some(re),
                    Err(_) => self.search_error = Some("invalid regex".into()),
                }
            }
        }
        self.refresh_rows();
    }

    fn clear_search(&mut self) {
        self.search_query.clear();
        self.update_search();
    }

    // ── Notifications ───────────────────────────────────────────────────

    pub fn toast(&mut self, kind: ToastKind, message: impl Into<String>) {
        let n = &config::get().notifications;
        let ms = match kind {
            ToastKind::Error => n.error_duration_ms,
            ToastKind::Success | ToastKind::Info => n.success_duration_ms,
        };
        let message = message.into();
        // Replace an identical toast instead of stacking duplicates.
        self.toasts.retain(|t| t.message != message);
        self.toasts.push_back(Toast {
            kind,
            message,
            shown: Instant::now(),
            ttl: Duration::from_millis(ms),
        });
        while self.toasts.len() > MAX_TOASTS {
            self.toasts.pop_front();
        }
        self.dirty = true;
    }

    // ── Ticks ───────────────────────────────────────────────────────────

    pub fn on_tick(&mut self) {
        let before = self.toasts.len();
        self.toasts.retain(|t| t.shown.elapsed() < t.ttl);
        if self.toasts.len() != before {
            self.dirty = true;
        }

        // BlueZ cancelled or timed out the request we are showing.
        if let Some(Modal::Prompt(p)) = &self.modal {
            if p.reply.as_ref().is_none_or(oneshot::Sender::is_closed) {
                self.modal = None;
                self.toast(ToastKind::Info, "Pairing request cancelled");
            }
        }

        if self.is_animating() {
            self.dirty = true;
        }
    }

    // ── Bluetooth events ────────────────────────────────────────────────

    pub fn handle_bt_event(&mut self, event: BtEvent) {
        self.dirty = true;
        match event {
            BtEvent::Ready => {
                if matches!(self.status, BtStatus::Unavailable(_)) {
                    self.toast(ToastKind::Success, "Bluetooth is back");
                }
                self.status = BtStatus::Ready;
            }
            BtEvent::Unavailable(msg) => {
                self.status = BtStatus::Unavailable(msg);
                self.devices.clear();
                self.pending.clear();
                self.scanning = false;
                self.adapter = AdapterInfo::default();
                if matches!(
                    self.modal,
                    Some(Modal::Prompt(_) | Modal::PairingCode { .. })
                ) {
                    self.modal = None;
                }
                self.refresh_rows();
            }
            BtEvent::AdapterState(info) => {
                self.adapter = info;
            }
            BtEvent::DeviceUpdated(info) => {
                match self.devices.iter_mut().find(|d| d.address == info.address) {
                    Some(existing) => *existing = info,
                    None => self.devices.push(info),
                }
                self.refresh_rows();
            }
            BtEvent::DeviceRemoved(addr) => {
                self.devices.retain(|d| d.address != addr);
                self.pending.remove(&addr);
                self.refresh_rows();
            }
            BtEvent::OpFinished { address, op, error } => self.op_finished(address, op, error),
            BtEvent::PairingCode { address, code } => {
                self.leave_text_mode();
                self.modal = Some(Modal::PairingCode { address, code });
            }
            BtEvent::AgentRequest(req) => self.open_prompt(req),
            BtEvent::ScanningChanged(scanning) => self.scanning = scanning,
            BtEvent::Error(msg) => self.toast(ToastKind::Error, msg),
        }
    }

    fn op_finished(&mut self, address: Address, op: Operation, error: Option<String>) {
        if self.pending.get(&address) == Some(&op) {
            self.pending.remove(&address);
        }
        if matches!(self.modal, Some(Modal::PairingCode { address: a, .. }) if a == address) {
            self.modal = None;
        }
        let name = self.name_of(address);
        match error {
            Some(err) => self.toast(
                ToastKind::Error,
                format!("Couldn't {} {name}: {err}", op.verb()),
            ),
            None => {
                let msg = match op {
                    Operation::Connect => format!("Connected to {name}"),
                    Operation::Disconnect => format!("Disconnected {name}"),
                    Operation::Pair => format!("Paired with {name}"),
                    Operation::Remove => format!("Forgot {name}"),
                    Operation::Rename => format!("Renamed to {name}"),
                    Operation::Trust => {
                        let trusted = self.device(address).is_some_and(|d| d.trusted);
                        if trusted {
                            format!("{name} is now trusted")
                        } else {
                            format!("{name} is no longer trusted")
                        }
                    }
                };
                self.toast(ToastKind::Success, msg);
            }
        }
    }

    fn open_prompt(&mut self, req: AgentRequest) {
        if matches!(self.modal, Some(Modal::Prompt(_))) {
            // One question at a time; dropping `req.reply` rejects this one.
            return;
        }
        self.leave_text_mode();
        self.modal = Some(Modal::Prompt(Prompt {
            address: req.address,
            kind: req.kind,
            reply: Some(req.reply),
            input: String::new(),
            error: None,
            opened: Instant::now(),
        }));
    }

    fn leave_text_mode(&mut self) {
        if self.mode == Mode::Rename {
            self.rename_target = None;
            self.rename_buffer.clear();
        }
        self.mode = Mode::Normal;
    }

    // ── Keyboard ────────────────────────────────────────────────────────

    /// Process a key press. Returns a command for the worker, if any.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<BtCommand> {
        self.dirty = true;
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            if let Some(Modal::Prompt(p)) = &mut self.modal {
                if let Some(reply) = p.reply.take() {
                    let _ = reply.send(AgentReply::Reject);
                }
            }
            self.running = false;
            return None;
        }
        if self.modal.is_some() {
            return self.handle_modal_key(key);
        }
        match self.mode {
            Mode::Normal => self.handle_normal_key(key),
            Mode::Search => {
                self.handle_search_key(key);
                None
            }
            Mode::Rename => self.handle_rename_key(key),
        }
    }

    /// Start a device operation, unless one is already running for it.
    fn start_op(&mut self, addr: Address, op: Operation, cmd: BtCommand) -> Option<BtCommand> {
        if let Some(running) = self.pending.get(&addr) {
            let msg = format!(
                "{} {}… please wait",
                running.progress_label(),
                self.name_of(addr)
            );
            self.toast(ToastKind::Info, msg);
            return None;
        }
        self.pending.insert(addr, op);
        Some(cmd)
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Option<BtCommand> {
        let kb = &config::get().keys;
        let code = key.code;

        // Navigation and UI-only keys work even while Bluetooth is down.
        match code {
            c if c == kb.quit => {
                self.running = false;
                return None;
            }
            c if c == kb.nav_down || c == KeyCode::Down => self.move_selection(1),
            c if c == kb.nav_up || c == KeyCode::Up => self.move_selection(-1),
            c if c == kb.jump_top || c == KeyCode::Home => self.select_edge(false),
            c if c == kb.jump_bottom || c == KeyCode::End => self.select_edge(true),
            KeyCode::PageDown => self.move_selection(PAGE),
            KeyCode::PageUp => self.move_selection(-PAGE),
            c if c == kb.help => self.modal = Some(Modal::Help),
            c if c == kb.search => self.mode = Mode::Search,
            c if c == kb.cycle_sort => {
                self.sort_mode = self.sort_mode.next();
                self.refresh_rows();
                self.toast(
                    ToastKind::Info,
                    format!("Sorted by {}", self.sort_mode.label()),
                );
            }
            KeyCode::Esc => {
                if self.filter_active() {
                    self.clear_search();
                } else {
                    self.toasts.clear();
                }
            }
            _ => return self.handle_bluetooth_key(code),
        }
        None
    }

    fn handle_bluetooth_key(&mut self, code: KeyCode) -> Option<BtCommand> {
        let kb = &config::get().keys;
        let device_keys = [
            kb.connect_toggle,
            kb.disconnect,
            kb.pair,
            kb.trust,
            kb.remove,
            kb.refresh,
            kb.rename,
        ];
        let is_bt_key =
            code == kb.toggle_adapter || code == kb.toggle_scan || device_keys.contains(&code);
        if !is_bt_key {
            self.dirty = false;
            return None;
        }
        if let BtStatus::Unavailable(msg) = &self.status {
            let msg = msg.clone();
            self.toast(ToastKind::Error, msg);
            return None;
        }
        if !self.is_ready() {
            return None;
        }

        if code == kb.toggle_adapter {
            return Some(if self.adapter.powered {
                BtCommand::DisableAdapter
            } else {
                BtCommand::EnableAdapter
            });
        }
        if code == kb.toggle_scan {
            return Some(if self.scanning {
                BtCommand::StopScan
            } else {
                BtCommand::StartScan
            });
        }

        // Everything below acts on the selected device.
        let Some(device) = self.selected_device().cloned() else {
            let hint = if self.devices.is_empty() {
                format!(
                    "No devices yet. Press {} to scan",
                    config::key_label(kb.toggle_scan)
                )
            } else {
                "Select a device first".into()
            };
            self.toast(ToastKind::Info, hint);
            return None;
        };
        if !self.adapter.powered {
            let msg = format!(
                "Bluetooth is off. Press {} to turn it on",
                config::key_label(kb.toggle_adapter)
            );
            self.toast(ToastKind::Info, msg);
            return None;
        }
        let addr = device.address;

        match code {
            c if c == kb.connect_toggle => {
                if device.connected {
                    self.start_op(addr, Operation::Disconnect, BtCommand::Disconnect(addr))
                } else {
                    self.start_op(addr, Operation::Connect, BtCommand::Connect(addr))
                }
            }
            c if c == kb.disconnect => {
                if device.connected {
                    self.start_op(addr, Operation::Disconnect, BtCommand::Disconnect(addr))
                } else {
                    self.toast(
                        ToastKind::Info,
                        format!("{} isn't connected", device.display_name()),
                    );
                    None
                }
            }
            c if c == kb.pair => {
                if device.paired {
                    self.toast(
                        ToastKind::Info,
                        format!("{} is already paired", device.display_name()),
                    );
                    None
                } else {
                    self.start_op(addr, Operation::Pair, BtCommand::Pair(addr))
                }
            }
            c if c == kb.trust => {
                self.start_op(addr, Operation::Trust, BtCommand::ToggleTrust(addr))
            }
            c if c == kb.remove => {
                self.modal = Some(Modal::ConfirmRemove(addr));
                None
            }
            c if c == kb.refresh => Some(BtCommand::RefreshDevice(addr)),
            c if c == kb.rename => {
                self.rename_target = Some(addr);
                self.rename_buffer = device.display_name().to_string();
                self.mode = Mode::Rename;
                None
            }
            _ => None,
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.clear_search();
            }
            KeyCode::Enter => self.mode = Mode::Normal,
            KeyCode::Down => self.move_selection(1),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Backspace => {
                self.search_query.pop();
                self.update_search();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.clear_search();
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search_query.push(c);
                self.update_search();
            }
            _ => {}
        }
    }

    fn handle_rename_key(&mut self, key: KeyEvent) -> Option<BtCommand> {
        match key.code {
            KeyCode::Esc => {
                self.leave_text_mode();
                None
            }
            KeyCode::Enter => {
                let target = self.rename_target.take();
                let alias = self.rename_buffer.trim().to_string();
                self.leave_text_mode();
                let addr = target?;
                if alias.is_empty() || self.device(addr).is_some_and(|d| d.display_name() == alias)
                {
                    return None;
                }
                self.start_op(addr, Operation::Rename, BtCommand::SetAlias(addr, alias))
            }
            KeyCode::Backspace => {
                self.rename_buffer.pop();
                None
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.rename_buffer.clear();
                None
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.rename_buffer.chars().count() < 64 {
                    self.rename_buffer.push(c);
                }
                None
            }
            _ => None,
        }
    }

    fn handle_modal_key(&mut self, key: KeyEvent) -> Option<BtCommand> {
        let kb = &config::get().keys;
        let yes = matches!(key.code, KeyCode::Enter | KeyCode::Char('y' | 'Y'));
        let no = matches!(key.code, KeyCode::Esc | KeyCode::Char('n' | 'N'));

        match self.modal.as_mut()? {
            Modal::Help => {
                if no || key.code == KeyCode::Enter || key.code == kb.help || key.code == kb.quit {
                    self.modal = None;
                }
                None
            }
            Modal::PairingCode { .. } => {
                if no || key.code == KeyCode::Enter {
                    self.modal = None;
                }
                None
            }
            Modal::ConfirmRemove(addr) => {
                let addr = *addr;
                if yes {
                    self.modal = None;
                    self.start_op(addr, Operation::Remove, BtCommand::RemoveDevice(addr))
                } else {
                    if no {
                        self.modal = None;
                    }
                    None
                }
            }
            Modal::Prompt(p) if p.needs_input() => {
                match key.code {
                    KeyCode::Esc => self.answer_prompt(AgentReply::Reject),
                    KeyCode::Enter => {
                        let valid = match p.kind {
                            PromptKind::EnterPasskey => agent::parse_passkey(&p.input).is_some(),
                            _ => agent::parse_pin(&p.input).is_some(),
                        };
                        if valid {
                            let text = p.input.clone();
                            self.answer_prompt(AgentReply::Text(text));
                        } else {
                            p.error = Some(match p.kind {
                                PromptKind::EnterPasskey => "Enter up to 6 digits".into(),
                                _ => "Enter 1–16 letters or digits".into(),
                            });
                        }
                    }
                    KeyCode::Backspace => {
                        p.input.pop();
                        p.error = None;
                    }
                    KeyCode::Char(c) => {
                        let limit = if p.kind == PromptKind::EnterPasskey {
                            6
                        } else {
                            16
                        };
                        let ok = if p.kind == PromptKind::EnterPasskey {
                            c.is_ascii_digit()
                        } else {
                            c.is_ascii_graphic()
                        };
                        if ok && p.input.len() < limit {
                            p.input.push(c);
                            p.error = None;
                        }
                    }
                    _ => {}
                }
                None
            }
            Modal::Prompt(_) => {
                if yes {
                    self.answer_prompt(AgentReply::Accept);
                } else if no {
                    self.answer_prompt(AgentReply::Reject);
                }
                None
            }
        }
    }

    fn answer_prompt(&mut self, answer: AgentReply) {
        if let Some(Modal::Prompt(mut p)) = self.modal.take() {
            if let Some(reply) = p.reply.take() {
                let _ = reply.send(answer);
            }
        }
    }

    // ── Mouse ───────────────────────────────────────────────────────────

    /// Process a mouse event. Returns a command for the worker, if any.
    pub fn handle_mouse(&mut self, ev: MouseEvent) -> Option<BtCommand> {
        if self.modal.is_some() {
            return None;
        }
        match ev.kind {
            MouseEventKind::ScrollDown => {
                self.move_selection(1);
                self.dirty = true;
                None
            }
            MouseEventKind::ScrollUp => {
                self.move_selection(-1);
                self.dirty = true;
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let area = self.list_area;
                let inside = ev.column >= area.x
                    && ev.column < area.x + area.width
                    && ev.row >= area.y
                    && ev.row < area.y + area.height;
                if !inside {
                    return None;
                }
                let row = self.list_state.offset() + usize::from(ev.row - area.y);
                let Some(Row::Device(i)) = self.rows.get(row).copied() else {
                    return None;
                };
                let addr = self.devices[i].address;
                self.selected = Some(addr);
                self.list_state.select(Some(row));
                self.dirty = true;

                let double = self
                    .last_click
                    .is_some_and(|(t, a)| a == addr && t.elapsed() < DOUBLE_CLICK);
                self.last_click = Some((Instant::now(), addr));
                if double {
                    self.last_click = None;
                    return self.handle_bluetooth_key(config::get().keys.connect_toggle);
                }
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventKind;

    fn setup() -> App {
        config::init_for_tests();
        let mut app = App::new();
        app.handle_bt_event(BtEvent::Ready);
        app.handle_bt_event(BtEvent::AdapterState(AdapterInfo {
            powered: true,
            ..AdapterInfo::default()
        }));
        app
    }

    fn dev(n: u8, name: &str, rssi: i16, paired: bool) -> DeviceInfo {
        let mut d = DeviceInfo::new(Address::new([0, 0, 0, 0, 0, n]));
        d.name = Some(name.into());
        d.alias = name.into();
        d.rssi = Some(rssi);
        d.paired = paired;
        d
    }

    fn press(app: &mut App, code: KeyCode) -> Option<BtCommand> {
        app.handle_key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        })
    }

    #[test]
    fn jump_keys_reach_both_ends() {
        let mut app = setup();
        for (n, name) in [(1, "A"), (2, "B"), (3, "C")] {
            app.handle_bt_event(BtEvent::DeviceUpdated(dev(
                n,
                name,
                -40 - i16::from(n),
                false,
            )));
        }
        press(&mut app, KeyCode::Char('G'));
        assert_eq!(app.selected_device().unwrap().alias, "C");
        press(&mut app, KeyCode::Char('g'));
        assert_eq!(app.selected_device().unwrap().alias, "A");
    }

    #[test]
    fn prompt_countdown_starts_near_timeout() {
        let mut app = setup();
        let (tx, _rx) = oneshot::channel();
        app.handle_bt_event(BtEvent::AgentRequest(AgentRequest {
            address: Address::new([9; 6]),
            kind: PromptKind::Authorize,
            reply: tx,
        }));
        let Some(Modal::Prompt(p)) = &app.modal else {
            panic!("no prompt")
        };
        assert!(p.seconds_left() >= 22, "{}", p.seconds_left());
    }

    #[test]
    fn groups_known_devices_first() {
        let mut app = setup();
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(1, "Speaker", -40, false)));
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(2, "Headphones", -80, true)));
        assert_eq!(app.rows[0], Row::Header(Section::Mine, 1));
        assert_eq!(app.rows[2], Row::Header(Section::Nearby, 1));
        assert_eq!(app.visible_count(), 2);
    }

    #[test]
    fn selection_follows_device_across_resorts() {
        let mut app = setup();
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(1, "A", -40, false)));
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(2, "B", -50, false)));
        press(&mut app, KeyCode::Char('j'));
        let chosen = app.selected.unwrap();
        assert_eq!(app.name_of(chosen), "B");
        // B's signal gets stronger, so it sorts above A.
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(2, "B", -30, false)));
        assert_eq!(app.selected, Some(chosen));
        let cmd = press(&mut app, KeyCode::Enter);
        assert_eq!(cmd, Some(BtCommand::Connect(chosen)));
    }

    #[test]
    fn failed_operation_clears_spinner_and_reports() {
        let mut app = setup();
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(1, "Buds", -40, false)));
        let addr = app.selected.unwrap();
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.pending.get(&addr), Some(&Operation::Connect));
        // A second Enter while busy doesn't send another command.
        assert_eq!(press(&mut app, KeyCode::Enter), None);
        app.handle_bt_event(BtEvent::OpFinished {
            address: addr,
            op: Operation::Connect,
            error: Some("nope".into()),
        });
        assert!(app.pending.is_empty());
        assert!(app
            .toasts
            .back()
            .unwrap()
            .message
            .contains("Couldn't connect to Buds"));
    }

    #[test]
    fn toasts_do_not_block_keys() {
        let mut app = setup();
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(1, "A", -40, false)));
        app.handle_bt_event(BtEvent::Error("boom".into()));
        assert!(press(&mut app, KeyCode::Char('s')).is_some());
    }

    #[test]
    fn remove_asks_for_confirmation() {
        let mut app = setup();
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(1, "A", -40, true)));
        let addr = app.selected.unwrap();
        assert_eq!(press(&mut app, KeyCode::Char('r')), None);
        assert!(matches!(app.modal, Some(Modal::ConfirmRemove(_))));
        assert_eq!(
            press(&mut app, KeyCode::Char('y')),
            Some(BtCommand::RemoveDevice(addr))
        );
    }

    #[test]
    fn pairing_prompt_waits_for_user() {
        let mut app = setup();
        let (tx, mut rx) = oneshot::channel();
        app.handle_bt_event(BtEvent::AgentRequest(AgentRequest {
            address: Address::new([9; 6]),
            kind: PromptKind::Confirm { passkey: 123_456 },
            reply: tx,
        }));
        // Unrelated keys don't answer it.
        press(&mut app, KeyCode::Char('s'));
        assert!(rx.try_recv().is_err());
        press(&mut app, KeyCode::Char('y'));
        assert_eq!(rx.try_recv().unwrap(), AgentReply::Accept);
        assert!(app.modal.is_none());
    }

    #[test]
    fn passkey_prompt_validates_input() {
        let mut app = setup();
        let (tx, mut rx) = oneshot::channel();
        app.handle_bt_event(BtEvent::AgentRequest(AgentRequest {
            address: Address::new([9; 6]),
            kind: PromptKind::EnterPasskey,
            reply: tx,
        }));
        press(&mut app, KeyCode::Enter);
        assert!(rx.try_recv().is_err(), "empty passkey must not be sent");
        for c in "12x34".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        press(&mut app, KeyCode::Enter);
        assert_eq!(rx.try_recv().unwrap(), AgentReply::Text("1234".into()));
    }

    #[test]
    fn search_filters_and_esc_restores() {
        let mut app = setup();
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(1, "Keyboard", -40, false)));
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(2, "Mouse", -50, false)));
        press(&mut app, KeyCode::Char('/'));
        for c in "mou".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        assert_eq!(app.visible_count(), 1);
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.visible_count(), 1, "filter stays after Enter");
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.visible_count(), 2);
    }

    #[test]
    fn invalid_regex_does_not_hide_everything() {
        let mut app = setup();
        app.handle_bt_event(BtEvent::DeviceUpdated(dev(1, "Keyboard", -40, false)));
        press(&mut app, KeyCode::Char('/'));
        for c in "/[".chars() {
            press(&mut app, KeyCode::Char(c));
        }
        assert!(app.search_error.is_some());
        assert_eq!(app.visible_count(), 1);
    }

    #[test]
    fn unavailable_bluetooth_explains_instead_of_sending() {
        let mut app = setup();
        app.handle_bt_event(BtEvent::Unavailable("service down".into()));
        assert_eq!(press(&mut app, KeyCode::Char('s')), None);
        assert_eq!(app.toasts.back().unwrap().message, "service down");
    }
}
