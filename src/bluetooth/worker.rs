//! Async Bluetooth worker task.
//!
//! Owns the `bluer::Session` and `Adapter`. Listens for `BtCommand`s from the
//! UI and emits `BtEvent`s back; the TUI thread never touches D-Bus.
//!
//! - Slow per-device operations (connect, pair, …) run as their own tasks, so
//!   one 30-second connect never stalls scanning or other commands.
//! - Every known device has a property stream, so changes made elsewhere
//!   (headphones auto-connecting, battery drain, signal) show up live.
//! - If BlueZ or the adapter goes away, the worker reports it and retries
//!   until it comes back.

use std::collections::HashMap;
use std::pin::Pin;
use std::time::Duration;

use bluer::agent::AgentHandle;
use bluer::{
    Adapter, AdapterEvent, AdapterProperty, Address, Device, DeviceEvent, DeviceProperty, Session,
};
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_stream::StreamMap;
use tracing::{debug, info, warn};

use super::agent;
use super::errors;
use super::types::{AdapterInfo, BtCommand, BtEvent, DeviceInfo, Operation};

type AdapterStream = Pin<Box<dyn Stream<Item = AdapterEvent> + Send>>;
type DeviceStream = Pin<Box<dyn Stream<Item = DeviceEvent> + Send>>;

/// How long to wait before retrying a failed BlueZ connection.
const RETRY_DELAY: Duration = Duration::from_secs(2);

/// RSSI jitter below this many dBm is not worth a UI update.
const RSSI_EPSILON: i16 = 3;

/// The main worker entry point. Runs until the command channel is closed
/// (i.e. the TUI exits).
pub async fn run(mut cmd_rx: mpsc::Receiver<BtCommand>, evt_tx: mpsc::Sender<BtEvent>) {
    let mut last_reported: Option<String> = None;

    loop {
        match Backend::connect(evt_tx.clone()).await {
            Ok(backend) => {
                last_reported = None;
                match backend.run(&mut cmd_rx).await {
                    Exit::UiClosed => return,
                    Exit::AdapterLost => {
                        let msg = format!(
                            "Lost the Bluetooth adapter. Waiting for it to come back… ({})",
                            errors::START_SERVICE_HINT
                        );
                        let _ = evt_tx.send(BtEvent::Unavailable(msg)).await;
                    }
                }
            }
            Err(msg) => {
                if last_reported.as_deref() != Some(msg.as_str()) {
                    warn!("Bluetooth unavailable: {msg}");
                    let _ = evt_tx.send(BtEvent::Unavailable(msg.clone())).await;
                    last_reported = Some(msg);
                }
            }
        }

        // Wait before retrying, but keep draining commands so the UI never
        // blocks and quits promptly.
        let retry = tokio::time::sleep(RETRY_DELAY);
        tokio::pin!(retry);
        loop {
            tokio::select! {
                () = &mut retry => break,
                cmd = cmd_rx.recv() => match cmd {
                    None => return,
                    Some(cmd) => {
                        debug!("Dropping {cmd:?}: Bluetooth unavailable");
                        let _ = evt_tx
                            .send(BtEvent::Error("Bluetooth isn't available right now".into()))
                            .await;
                    }
                },
            }
        }
    }
}

enum Exit {
    UiClosed,
    AdapterLost,
}

/// What woke the main loop.
enum Wake {
    Command(Option<BtCommand>),
    Adapter(Option<AdapterEvent>),
    Discovery(Option<AdapterEvent>),
    Device(Address, DeviceEvent),
    OpDone,
}

struct Backend {
    _session: Session,
    adapter: Adapter,
    _agent: Option<AgentHandle>,
    evt_tx: mpsc::Sender<BtEvent>,
    /// Last snapshot sent to the UI, per device.
    devices: HashMap<Address, DeviceInfo>,
    /// Live property-change streams, one per device.
    device_streams: StreamMap<Address, DeviceStream>,
    /// Active discovery session; dropping it stops discovery.
    discovery: Option<AdapterStream>,
    /// In-flight per-device operations. Dropping the set aborts them.
    ops: JoinSet<()>,
}

impl Backend {
    async fn connect(evt_tx: mpsc::Sender<BtEvent>) -> Result<Self, String> {
        let session = Session::new().await.map_err(|e| errors::unavailable(&e))?;
        let adapter = session
            .default_adapter()
            .await
            .map_err(|e| errors::unavailable(&e))?;
        // Touch a property: `default_adapter` can succeed on a stale name.
        adapter
            .is_powered()
            .await
            .map_err(|e| errors::unavailable(&e))?;

        let agent = match agent::register(&session, evt_tx.clone()).await {
            Ok(handle) => Some(handle),
            Err(e) => {
                warn!("Failed to register pairing agent: {e}");
                let _ = evt_tx
                    .send(BtEvent::Error(
                        "Couldn't register the pairing agent; devices that need a code may fail to pair"
                            .into(),
                    ))
                    .await;
                None
            }
        };

        info!("Using adapter {}", adapter.name());
        Ok(Self {
            _session: session,
            adapter,
            _agent: agent,
            evt_tx,
            devices: HashMap::new(),
            device_streams: StreamMap::new(),
            discovery: None,
            ops: JoinSet::new(),
        })
    }

    async fn run(mut self, cmd_rx: &mut mpsc::Receiver<BtCommand>) -> Exit {
        let mut adapter_events: AdapterStream = match self.adapter.events().await {
            Ok(stream) => Box::pin(stream),
            Err(e) => {
                warn!("Cannot subscribe to adapter events: {e}");
                return Exit::AdapterLost;
            }
        };

        self.send(BtEvent::Ready).await;
        self.emit_adapter_state().await;
        self.load_known_devices().await;

        if crate::config::get().general.scan_on_startup {
            self.start_scan().await;
        }

        loop {
            let wake = tokio::select! {
                cmd = cmd_rx.recv() => Wake::Command(cmd),
                evt = adapter_events.next() => Wake::Adapter(evt),
                evt = next_discovery(&mut self.discovery) => Wake::Discovery(evt),
                Some((addr, evt)) = self.device_streams.next(), if !self.device_streams.is_empty() => {
                    Wake::Device(addr, evt)
                }
                Some(_) = self.ops.join_next(), if !self.ops.is_empty() => Wake::OpDone,
            };

            match wake {
                Wake::Command(None) => return Exit::UiClosed,
                Wake::Command(Some(cmd)) => self.handle_command(cmd).await,
                Wake::Adapter(None) => return Exit::AdapterLost,
                Wake::Adapter(Some(evt)) | Wake::Discovery(Some(evt)) => {
                    self.handle_adapter_event(evt).await;
                }
                Wake::Discovery(None) => {
                    self.discovery = None;
                    self.send(BtEvent::ScanningChanged(false)).await;
                }
                Wake::Device(addr, DeviceEvent::PropertyChanged(prop)) => {
                    self.apply_property(addr, prop).await;
                }
                Wake::OpDone => {}
            }
        }
    }

    async fn send(&mut self, evt: BtEvent) {
        let _ = self.evt_tx.send(evt).await;
    }

    async fn error(&mut self, msg: impl Into<String>) {
        self.send(BtEvent::Error(msg.into())).await;
    }

    async fn emit_adapter_state(&mut self) {
        let a = &self.adapter;
        let (address, powered, discovering, alias) =
            tokio::join!(a.address(), a.is_powered(), a.is_discovering(), a.alias());
        let info = AdapterInfo {
            name: a.name().to_string(),
            alias: alias.unwrap_or_default(),
            address: address.ok(),
            powered: powered.unwrap_or(false),
            discovering: discovering.unwrap_or(false),
        };
        self.send(BtEvent::AdapterState(info)).await;
    }

    async fn load_known_devices(&mut self) {
        let Ok(addrs) = self.adapter.device_addresses().await else {
            return;
        };
        for addr in addrs {
            self.track(addr).await;
        }
    }

    /// Start following a device: snapshot it, subscribe to its changes.
    async fn track(&mut self, addr: Address) {
        if self.devices.contains_key(&addr) {
            return;
        }
        let Ok(device) = self.adapter.device(addr) else {
            return;
        };
        if let Ok(events) = device.events().await {
            self.device_streams.insert(addr, Box::pin(events));
        }
        let info = snapshot_device(&device).await;
        self.devices.insert(addr, info.clone());
        self.send(BtEvent::DeviceUpdated(info)).await;
    }

    fn forget(&mut self, addr: Address) {
        self.devices.remove(&addr);
        self.device_streams.remove(&addr);
    }

    async fn apply_property(&mut self, addr: Address, prop: DeviceProperty) {
        let Some(info) = self.devices.get_mut(&addr) else {
            return;
        };
        if apply_property(info, prop) {
            let info = info.clone();
            self.send(BtEvent::DeviceUpdated(info)).await;
        }
    }

    async fn handle_adapter_event(&mut self, evt: AdapterEvent) {
        match evt {
            AdapterEvent::DeviceAdded(addr) => self.track(addr).await,
            AdapterEvent::DeviceRemoved(addr) => {
                self.forget(addr);
                self.send(BtEvent::DeviceRemoved(addr)).await;
            }
            AdapterEvent::PropertyChanged(prop) => {
                if matches!(prop, AdapterProperty::Powered(false)) && self.discovery.is_some() {
                    self.discovery = None;
                    self.send(BtEvent::ScanningChanged(false)).await;
                }
                self.emit_adapter_state().await;
            }
        }
    }

    async fn start_scan(&mut self) {
        if self.discovery.is_some() {
            return;
        }
        if !self.adapter.is_powered().await.unwrap_or(false) {
            if let Err(e) = self.adapter.set_powered(true).await {
                self.error(format!(
                    "Couldn't turn Bluetooth on: {}",
                    errors::humanize(&e)
                ))
                .await;
                return;
            }
        }
        match self.adapter.discover_devices().await {
            Ok(stream) => {
                self.discovery = Some(Box::pin(stream));
                self.send(BtEvent::ScanningChanged(true)).await;
                info!("Discovery started");
            }
            Err(e) => {
                self.error(format!("Couldn't start scanning: {}", errors::humanize(&e)))
                    .await;
            }
        }
    }

    async fn handle_command(&mut self, cmd: BtCommand) {
        match cmd {
            BtCommand::EnableAdapter | BtCommand::DisableAdapter => {
                let on = cmd == BtCommand::EnableAdapter;
                if !on && self.discovery.take().is_some() {
                    self.send(BtEvent::ScanningChanged(false)).await;
                }
                if let Err(e) = self.adapter.set_powered(on).await {
                    let what = if on {
                        "turn Bluetooth on"
                    } else {
                        "turn Bluetooth off"
                    };
                    self.error(format!("Couldn't {what}: {}", errors::humanize(&e)))
                        .await;
                }
                self.emit_adapter_state().await;
            }
            BtCommand::StartScan => self.start_scan().await,
            BtCommand::StopScan => {
                if self.discovery.take().is_some() {
                    info!("Discovery stopped");
                }
                self.send(BtEvent::ScanningChanged(false)).await;
            }
            BtCommand::RefreshDevice(addr) => {
                if let Ok(device) = self.adapter.device(addr) {
                    let info = snapshot_device(&device).await;
                    self.devices.insert(addr, info.clone());
                    self.send(BtEvent::DeviceUpdated(info)).await;
                }
            }
            BtCommand::Connect(addr) => self.spawn_op(addr, Operation::Connect, None),
            BtCommand::Disconnect(addr) => self.spawn_op(addr, Operation::Disconnect, None),
            BtCommand::Pair(addr) => self.spawn_op(addr, Operation::Pair, None),
            BtCommand::ToggleTrust(addr) => self.spawn_op(addr, Operation::Trust, None),
            BtCommand::RemoveDevice(addr) => self.spawn_op(addr, Operation::Remove, None),
            BtCommand::SetAlias(addr, alias) => self.spawn_op(addr, Operation::Rename, Some(alias)),
        }
    }

    /// Run a device operation in its own task; it always reports `OpFinished`.
    fn spawn_op(&mut self, address: Address, op: Operation, arg: Option<String>) {
        let adapter = self.adapter.clone();
        let tx = self.evt_tx.clone();
        self.ops.spawn(async move {
            let result = run_op(&adapter, address, op, arg).await;
            let error = result.err().map(|e| errors::humanize(&e));
            if let Some(err) = &error {
                warn!("{op:?} {address} failed: {err}");
            }
            let _ = tx.send(BtEvent::OpFinished { address, op, error }).await;
        });
    }
}

async fn next_discovery(discovery: &mut Option<AdapterStream>) -> Option<AdapterEvent> {
    match discovery {
        Some(stream) => stream.next().await,
        None => std::future::pending().await,
    }
}

async fn run_op(
    adapter: &Adapter,
    address: Address,
    op: Operation,
    arg: Option<String>,
) -> bluer::Result<()> {
    if op == Operation::Remove {
        return adapter.remove_device(address).await;
    }
    let device = adapter.device(address)?;
    match op {
        Operation::Connect => connect_lifecycle(&device).await,
        Operation::Disconnect => device.disconnect().await,
        Operation::Pair => match device.pair().await {
            Err(e) if e.kind == bluer::ErrorKind::AlreadyExists => Ok(()),
            other => other,
        },
        Operation::Trust => {
            let trusted = device.is_trusted().await?;
            device.set_trusted(!trusted).await
        }
        Operation::Rename => device.set_alias(arg.unwrap_or_default()).await,
        Operation::Remove => unreachable!("handled above"),
    }
}

/// Full connection lifecycle: pair (if needed) → trust → connect.
/// Respects the `auto_trust_on_pair` and `connection_timeout_secs` config.
async fn connect_lifecycle(device: &Device) -> bluer::Result<()> {
    let cfg = &crate::config::get().bluetooth;
    let timeout = Duration::from_secs(cfg.connection_timeout_secs);

    let steps = async {
        if !device.is_paired().await.unwrap_or(false) {
            info!("Pairing with {}…", device.address());
            match device.pair().await {
                Err(e) if e.kind == bluer::ErrorKind::AlreadyExists => {}
                other => other?,
            }
        }
        if cfg.auto_trust_on_pair && !device.is_trusted().await.unwrap_or(false) {
            device.set_trusted(true).await?;
        }
        info!("Connecting to {}…", device.address());
        match device.connect().await {
            Err(e) if e.kind == bluer::ErrorKind::AlreadyConnected => Ok(()),
            other => other,
        }
    };

    tokio::time::timeout(timeout, steps)
        .await
        .unwrap_or_else(|_| {
            Err(bluer::Error {
                kind: bluer::ErrorKind::Failed,
                message: format!(
                    "Timed out after {}s. Make sure the device is on and in range",
                    cfg.connection_timeout_secs
                ),
            })
        })
}

/// Read every interesting property of a device concurrently.
async fn snapshot_device(device: &Device) -> DeviceInfo {
    let (name, alias, icon, rssi, paired, trusted, connected, class, battery) = tokio::join!(
        device.name(),
        device.alias(),
        device.icon(),
        device.rssi(),
        device.is_paired(),
        device.is_trusted(),
        device.is_connected(),
        device.class(),
        device.battery_percentage(),
    );
    let mut info = DeviceInfo::new(device.address());
    info.name = name.ok().flatten();
    if let Ok(alias) = alias {
        info.alias = alias;
    }
    info.icon = icon.ok().flatten();
    info.rssi = rssi.ok().flatten();
    info.paired = paired.unwrap_or(false);
    info.trusted = trusted.unwrap_or(false);
    info.connected = connected.unwrap_or(false);
    info.class = class.ok().flatten();
    info.battery = battery.ok().flatten();
    info
}

/// Apply a property change to a snapshot. Returns whether the UI-visible
/// state changed enough to be worth sending.
fn apply_property(info: &mut DeviceInfo, prop: DeviceProperty) -> bool {
    fn set<T: PartialEq>(slot: &mut T, value: T) -> bool {
        if *slot == value {
            false
        } else {
            *slot = value;
            true
        }
    }
    match prop {
        DeviceProperty::Name(v) => set(&mut info.name, Some(v)),
        DeviceProperty::Alias(v) => set(&mut info.alias, v),
        DeviceProperty::Icon(v) => set(&mut info.icon, Some(v)),
        DeviceProperty::Class(v) => set(&mut info.class, Some(v)),
        DeviceProperty::Paired(v) => set(&mut info.paired, v),
        DeviceProperty::Trusted(v) => set(&mut info.trusted, v),
        DeviceProperty::Connected(v) => set(&mut info.connected, v),
        DeviceProperty::BatteryPercentage(v) => set(&mut info.battery, Some(v)),
        DeviceProperty::Rssi(v) => match info.rssi {
            Some(old) if (old - v).abs() < RSSI_EPSILON => false,
            _ => set(&mut info.rssi, Some(v)),
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> DeviceInfo {
        DeviceInfo::new(Address::new([1, 2, 3, 4, 5, 6]))
    }

    #[test]
    fn small_rssi_jitter_is_ignored() {
        let mut d = device();
        assert!(apply_property(&mut d, DeviceProperty::Rssi(-60)));
        assert!(!apply_property(&mut d, DeviceProperty::Rssi(-62)));
        assert!(apply_property(&mut d, DeviceProperty::Rssi(-70)));
        assert_eq!(d.rssi, Some(-70));
    }

    #[test]
    fn unchanged_values_do_not_trigger_updates() {
        let mut d = device();
        assert!(apply_property(&mut d, DeviceProperty::Connected(true)));
        assert!(!apply_property(&mut d, DeviceProperty::Connected(true)));
        assert!(!apply_property(
            &mut d,
            DeviceProperty::ServicesResolved(true)
        ));
    }

    #[test]
    fn rename_updates_display_name() {
        let mut d = device();
        d.name = Some("WH-1000XM5".into());
        apply_property(&mut d, DeviceProperty::Alias("My Headphones".into()));
        assert_eq!(d.display_name(), "My Headphones");
    }
}
