//! Async Bluetooth worker task.
//!
//! Owns the `bluer::Session` and `Adapter`. Listens for `BtCommand`s from the
//! UI and emits `BtEvent`s back. Runs entirely on the tokio runtime — the TUI
//! thread never touches D-Bus.

use std::collections::HashSet;

use bluer::{Adapter, AdapterEvent, Address, Device, Session};
use futures::StreamExt;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::agent;
use super::types::*;

/// Snapshot all interesting properties from a `bluer::Device` into a plain
/// `DeviceInfo` struct that can be sent across the channel.
async fn snapshot_device(device: &Device) -> DeviceInfo {
    // Every property access is a D-Bus call that can fail. We treat failures
    // as "unknown" rather than propagating errors.
    let address = device.address();
    let name = device.name().await.unwrap_or(None);
    let alias = device.alias().await.unwrap_or_else(|_| address.to_string());
    let icon = device.icon().await.unwrap_or(None);
    let rssi = device.rssi().await.unwrap_or(None);
    let tx_power = device.tx_power().await.unwrap_or(None);
    let paired = device.is_paired().await.unwrap_or(false);
    let trusted = device.is_trusted().await.unwrap_or(false);
    let connected = device.is_connected().await.unwrap_or(false);
    let class = device.class().await.unwrap_or(None);
    let battery = device.battery_percentage().await.unwrap_or(None);

    DeviceInfo {
        address,
        name,
        alias,
        icon,
        rssi,
        tx_power,
        battery,
        paired,
        trusted,
        connected,
        class,
    }
}

/// Send the current adapter state to the UI.
async fn emit_adapter_state(adapter: &Adapter, tx: &mpsc::Sender<BtEvent>) {
    let info = AdapterInfo {
        name: adapter.name().to_string(),
        address: adapter.address().await.ok(),
        powered: adapter.is_powered().await.unwrap_or(false),
        discovering: adapter.is_discovering().await.unwrap_or(false),
        discoverable: adapter.is_discoverable().await.unwrap_or(false),
    };
    let _ = tx.send(BtEvent::AdapterState(info)).await;
}

/// The main worker entry point. Runs until the command channel is closed
/// (i.e. the TUI exits).
pub async fn run(mut cmd_rx: mpsc::Receiver<BtCommand>, evt_tx: mpsc::Sender<BtEvent>) {
    // ── Session & adapter initialisation ────────────────────────────────
    let session = match Session::new().await {
        Ok(s) => s,
        Err(e) => {
            let _ = evt_tx
                .send(BtEvent::Error(format!(
                    "Failed to connect to BlueZ D-Bus: {e}"
                )))
                .await;
            return;
        }
    };

    // Register our custom agent so pairing PIN/passkey prompts are forwarded
    // to the TUI instead of being silently handled (or failing) via the
    // default BlueZ agent.
    let _agent_handle = match agent::register(&session, evt_tx.clone()).await {
        Ok(h) => Some(h),
        Err(e) => {
            warn!("Failed to register BT agent (pairing may not work): {e}");
            None
        }
    };

    let adapter = match session.default_adapter().await {
        Ok(a) => a,
        Err(e) => {
            let _ = evt_tx
                .send(BtEvent::Error(format!("No Bluetooth adapter found: {e}")))
                .await;
            return;
        }
    };

    info!("Using adapter: {}", adapter.name());
    emit_adapter_state(&adapter, &evt_tx).await;

    // Send initial list of already-known devices.
    if let Ok(addrs) = adapter.device_addresses().await {
        for addr in addrs {
            if let Ok(device) = adapter.device(addr) {
                let info = snapshot_device(&device).await;
                let _ = evt_tx.send(BtEvent::DeviceFound(info)).await;
            }
        }
    }

    // ── Discovery stream (optional — started/stopped by commands) ───────
    // We hold the discovery stream in an Option so we can start/stop it.
    let mut discover_stream: Option<
        std::pin::Pin<Box<dyn futures::Stream<Item = AdapterEvent> + Send>>,
    > = None;

    // Track which addresses we've already sent DeviceFound for so we can
    // send DeviceUpdated on subsequent sightings.
    let mut known_addresses: HashSet<Address> = HashSet::new();

    // ── Main select loop ────────────────────────────────────────────────
    loop {
        tokio::select! {
            // ── Commands from UI ────────────────────────────────────────
            cmd = cmd_rx.recv() => {
                let Some(cmd) = cmd else {
                    debug!("Command channel closed — worker exiting");
                    break;
                };
                handle_command(
                    &cmd,
                    &adapter,
                    &evt_tx,
                    &mut discover_stream,
                    &mut known_addresses,
                )
                .await;
            }

            // ── Discovery events ────────────────────────────────────────
            Some(adapter_event) = async {
                match discover_stream.as_mut() {
                    Some(stream) => stream.next().await,
                    None => std::future::pending::<Option<AdapterEvent>>().await,
                }
            } => {
                handle_adapter_event(
                    adapter_event,
                    &adapter,
                    &evt_tx,
                    &mut known_addresses,
                )
                .await;
            }
        }
    }

    info!("Bluetooth worker shut down");
}

/// Process a single command from the UI.
async fn handle_command(
    cmd: &BtCommand,
    adapter: &Adapter,
    evt_tx: &mpsc::Sender<BtEvent>,
    discover_stream: &mut Option<
        std::pin::Pin<Box<dyn futures::Stream<Item = AdapterEvent> + Send>>,
    >,
    known_addresses: &mut HashSet<Address>,
) {
    match cmd {
        BtCommand::EnableAdapter => {
            if let Err(e) = adapter.set_powered(true).await {
                let _ = evt_tx
                    .send(BtEvent::Error(format!("Failed to enable adapter: {e}")))
                    .await;
            }
            emit_adapter_state(adapter, evt_tx).await;
        }

        BtCommand::DisableAdapter => {
            if let Err(e) = adapter.set_powered(false).await {
                let _ = evt_tx
                    .send(BtEvent::Error(format!("Failed to disable adapter: {e}")))
                    .await;
            }
            emit_adapter_state(adapter, evt_tx).await;
        }

        BtCommand::StartScan => {
            match adapter.discover_devices().await {
                Ok(stream) => {
                    *discover_stream = Some(Box::pin(stream));
                    let _ = evt_tx.send(BtEvent::ScanningChanged(true)).await;
                    info!("Discovery started");
                }
                Err(e) => {
                    let _ = evt_tx
                        .send(BtEvent::Error(format!("Failed to start scanning: {e}")))
                        .await;
                }
            }
            emit_adapter_state(adapter, evt_tx).await;
        }

        BtCommand::StopScan => {
            *discover_stream = None;
            let _ = evt_tx.send(BtEvent::ScanningChanged(false)).await;
            emit_adapter_state(adapter, evt_tx).await;
            info!("Discovery stopped");
        }

        BtCommand::Connect(addr) => {
            let addr = *addr;
            let adapter_name = adapter.name().to_string();
            let evt_tx = evt_tx.clone();

            // Get the device handle.
            match adapter.device(addr) {
                Ok(device) => {
                    // Pair → Trust → Connect lifecycle.
                    let result = connect_lifecycle(&device).await;
                    match result {
                        Ok(()) => {
                            let info = snapshot_device(&device).await;
                            let _ = evt_tx.send(BtEvent::DeviceUpdated(info)).await;
                            let _ = evt_tx
                                .send(BtEvent::ConnectionResult {
                                    address: addr,
                                    success: true,
                                    error: None,
                                })
                                .await;
                        }
                        Err(e) => {
                            let _ = evt_tx
                                .send(BtEvent::ConnectionResult {
                                    address: addr,
                                    success: false,
                                    error: Some(e.to_string()),
                                })
                                .await;
                        }
                    }
                }
                Err(e) => {
                    let _ = evt_tx
                        .send(BtEvent::ConnectionResult {
                            address: addr,
                            success: false,
                            error: Some(format!("Device not found on {adapter_name}: {e}")),
                        })
                        .await;
                }
            }
        }

        BtCommand::Disconnect(addr) => {
            let addr = *addr;
            match adapter.device(addr) {
                Ok(device) => {
                    if let Err(e) = device.disconnect().await {
                        let _ = evt_tx
                            .send(BtEvent::Error(format!("Disconnect failed: {e}")))
                            .await;
                    }
                    let info = snapshot_device(&device).await;
                    let _ = evt_tx.send(BtEvent::DeviceUpdated(info)).await;
                }
                Err(e) => {
                    let _ = evt_tx
                        .send(BtEvent::Error(format!("Device not found: {e}")))
                        .await;
                }
            }
        }

        BtCommand::Pair(addr) => {
            let addr = *addr;
            match adapter.device(addr) {
                Ok(device) => match device.pair().await {
                    Ok(()) => {
                        let info = snapshot_device(&device).await;
                        let _ = evt_tx.send(BtEvent::DeviceUpdated(info)).await;
                        let _ = evt_tx
                            .send(BtEvent::PairResult {
                                address: addr,
                                success: true,
                                error: None,
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = evt_tx
                            .send(BtEvent::PairResult {
                                address: addr,
                                success: false,
                                error: Some(e.to_string()),
                            })
                            .await;
                    }
                },
                Err(e) => {
                    let _ = evt_tx
                        .send(BtEvent::PairResult {
                            address: addr,
                            success: false,
                            error: Some(format!("Device not found: {e}")),
                        })
                        .await;
                }
            }
        }

        BtCommand::Trust(addr) => {
            let addr = *addr;
            match adapter.device(addr) {
                Ok(device) => {
                    let currently_trusted = device.is_trusted().await.unwrap_or(false);
                    if let Err(e) = device.set_trusted(!currently_trusted).await {
                        let _ = evt_tx
                            .send(BtEvent::Error(format!("Failed to toggle trust: {e}")))
                            .await;
                    }
                    let info = snapshot_device(&device).await;
                    let _ = evt_tx.send(BtEvent::DeviceUpdated(info)).await;
                }
                Err(e) => {
                    let _ = evt_tx
                        .send(BtEvent::Error(format!("Device not found: {e}")))
                        .await;
                }
            }
        }

        BtCommand::RemoveDevice(addr) => {
            let addr = *addr;
            if let Err(e) = adapter.remove_device(addr).await {
                let _ = evt_tx
                    .send(BtEvent::Error(format!("Failed to remove device: {e}")))
                    .await;
            } else {
                let _ = evt_tx.send(BtEvent::DeviceRemoved(addr)).await;
                known_addresses.remove(&addr);
            }
        }

        BtCommand::RefreshDevice(addr) => {
            let addr = *addr;
            match adapter.device(addr) {
                Ok(device) => {
                    let info = snapshot_device(&device).await;
                    let _ = evt_tx.send(BtEvent::DeviceUpdated(info)).await;
                }
                Err(e) => {
                    let _ = evt_tx
                        .send(BtEvent::Error(format!("Device not found: {e}")))
                        .await;
                }
            }
        }
    }
}

/// Handle a single adapter discovery event.
async fn handle_adapter_event(
    event: AdapterEvent,
    adapter: &Adapter,
    evt_tx: &mpsc::Sender<BtEvent>,
    known_addresses: &mut HashSet<Address>,
) {
    match event {
        AdapterEvent::DeviceAdded(addr) => {
            if let Ok(device) = adapter.device(addr) {
                let info = snapshot_device(&device).await;
                if known_addresses.insert(addr) {
                    let _ = evt_tx.send(BtEvent::DeviceFound(info)).await;
                } else {
                    let _ = evt_tx.send(BtEvent::DeviceUpdated(info)).await;
                }
            }
        }
        AdapterEvent::DeviceRemoved(addr) => {
            known_addresses.remove(&addr);
            let _ = evt_tx.send(BtEvent::DeviceRemoved(addr)).await;
        }
        AdapterEvent::PropertyChanged(_prop) => {
            emit_adapter_state(adapter, evt_tx).await;
        }
    }
}

/// Full connection lifecycle: pair (if needed) → trust → connect.
/// Respects the `auto_trust_on_pair` and `connection_timeout_secs` config.
async fn connect_lifecycle(device: &Device) -> bluer::Result<()> {
    let bt_cfg = &crate::config::get().bluetooth;
    let timeout = std::time::Duration::from_secs(bt_cfg.connection_timeout_secs);

    let fut = async {
        // Step 1: Pair if not already paired.
        if !device.is_paired().await.unwrap_or(false) {
            info!("Pairing with {}…", device.address());
            device.pair().await?;
        }

        // Step 2: Trust if configured and not already trusted.
        if bt_cfg.auto_trust_on_pair && !device.is_trusted().await.unwrap_or(false) {
            info!("Trusting {}…", device.address());
            device.set_trusted(true).await?;
        }

        // Step 3: Connect.
        info!("Connecting to {}…", device.address());
        device.connect().await?;

        info!("Connected to {}", device.address());
        Ok(())
    };

    match tokio::time::timeout(timeout, fut).await {
        Ok(result) => result,
        Err(_) => Err(bluer::Error {
            kind: bluer::ErrorKind::Failed,
            message: format!(
                "Connection timed out after {}s",
                bt_cfg.connection_timeout_secs
            ),
        }),
    }
}
