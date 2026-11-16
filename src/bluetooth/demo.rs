//! Scripted fake adapter for screenshots, recordings and UI work without
//! hardware. Only compiled with `--features demo`, and only used when
//! `VOIDLINK_DEMO=1` is set. Release and AUR builds never include it.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
use tokio::time::sleep;

use super::types::{
    AdapterInfo, Address, AgentReply, AgentRequest, BtCommand, BtEvent, DeviceInfo, Operation,
    PromptKind,
};

type Devices = Arc<Mutex<HashMap<Address, DeviceInfo>>>;

fn addr(last: u8) -> Address {
    Address::new([0xD0, 0x5E, 0xC0, 0xDE, 0x00, last])
}

#[allow(clippy::too_many_arguments)]
fn device(
    n: u8,
    name: Option<&str>,
    icon: &str,
    rssi: Option<i16>,
    battery: Option<u8>,
    paired: bool,
    connected: bool,
) -> DeviceInfo {
    let mut d = DeviceInfo::new(addr(n));
    d.name = name.map(Into::into);
    d.alias = name.map_or_else(|| d.address.to_string().replace(':', "-"), Into::into);
    d.icon = Some(icon.into());
    d.rssi = rssi;
    d.battery = battery;
    d.paired = paired;
    d.trusted = paired;
    d.connected = connected;
    d
}

fn known() -> Vec<DeviceInfo> {
    vec![
        device(
            1,
            Some("Sony WH-1000XM5"),
            "audio-headset",
            Some(-47),
            Some(80),
            true,
            true,
        ),
        device(
            2,
            Some("MX Master 3S"),
            "input-mouse",
            Some(-52),
            Some(64),
            true,
            true,
        ),
        device(
            3,
            Some("Keychron K2"),
            "input-keyboard",
            None,
            None,
            true,
            false,
        ),
    ]
}

fn nearby() -> Vec<DeviceInfo> {
    vec![
        device(
            10,
            Some("JBL Flip 6"),
            "audio-card",
            Some(-61),
            None,
            false,
            false,
        ),
        device(
            11,
            Some("Pixel 9 Pro"),
            "phone",
            Some(-66),
            None,
            false,
            false,
        ),
        device(
            12,
            Some("Xbox Wireless Controller"),
            "input-gaming",
            Some(-71),
            None,
            false,
            false,
        ),
        device(
            13,
            Some("Galaxy Buds2 Pro"),
            "audio-headset",
            Some(-77),
            None,
            false,
            false,
        ),
        device(14, None, "", Some(-89), None, false, false),
    ]
}

pub async fn run(mut cmd_rx: mpsc::Receiver<BtCommand>, tx: mpsc::Sender<BtEvent>) {
    let devices: Devices = Arc::default();
    let mut adapter = AdapterInfo {
        name: "hci0".into(),
        alias: "voidlink-demo".into(),
        address: Some(Address::new([0xD0, 0x5E, 0xC0, 0xDE, 0xFF, 0x01])),
        powered: true,
        discovering: false,
    };

    sleep(Duration::from_millis(300)).await;
    let _ = tx.send(BtEvent::Ready).await;
    let _ = tx.send(BtEvent::AdapterState(adapter.clone())).await;
    for d in known() {
        devices.lock().unwrap().insert(d.address, d.clone());
        let _ = tx.send(BtEvent::DeviceUpdated(d)).await;
    }

    let mut scan: Option<tokio::task::JoinHandle<()>> = None;
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            BtCommand::EnableAdapter | BtCommand::DisableAdapter => {
                adapter.powered = cmd == BtCommand::EnableAdapter;
                if !adapter.powered {
                    if let Some(s) = scan.take() {
                        s.abort();
                        let _ = tx.send(BtEvent::ScanningChanged(false)).await;
                    }
                    let updates: Vec<DeviceInfo> = {
                        let mut map = devices.lock().unwrap();
                        map.values_mut()
                            .filter(|d| d.connected)
                            .map(|d| {
                                d.connected = false;
                                d.clone()
                            })
                            .collect()
                    };
                    for d in updates {
                        let _ = tx.send(BtEvent::DeviceUpdated(d)).await;
                    }
                }
                let _ = tx.send(BtEvent::AdapterState(adapter.clone())).await;
            }
            BtCommand::StartScan => {
                if scan.is_none() {
                    let _ = tx.send(BtEvent::ScanningChanged(true)).await;
                    scan = Some(tokio::spawn(scan_task(devices.clone(), tx.clone())));
                }
            }
            BtCommand::StopScan => {
                if let Some(s) = scan.take() {
                    s.abort();
                }
                let _ = tx.send(BtEvent::ScanningChanged(false)).await;
            }
            BtCommand::RefreshDevice(a) => {
                let d = devices.lock().unwrap().get(&a).cloned();
                if let Some(d) = d {
                    let _ = tx.send(BtEvent::DeviceUpdated(d)).await;
                }
            }
            BtCommand::Connect(a) => spawn_op(&devices, &tx, a, Operation::Connect, None),
            BtCommand::Disconnect(a) => spawn_op(&devices, &tx, a, Operation::Disconnect, None),
            BtCommand::Pair(a) => spawn_op(&devices, &tx, a, Operation::Pair, None),
            BtCommand::ToggleTrust(a) => spawn_op(&devices, &tx, a, Operation::Trust, None),
            BtCommand::RemoveDevice(a) => spawn_op(&devices, &tx, a, Operation::Remove, None),
            BtCommand::SetAlias(a, s) => spawn_op(&devices, &tx, a, Operation::Rename, Some(s)),
        }
    }
}

async fn scan_task(devices: Devices, tx: mpsc::Sender<BtEvent>) {
    for d in nearby() {
        sleep(Duration::from_millis(650)).await;
        let exists = devices.lock().unwrap().contains_key(&d.address);
        if !exists {
            devices.lock().unwrap().insert(d.address, d.clone());
            let _ = tx.send(BtEvent::DeviceUpdated(d)).await;
        }
    }
    // Gentle signal drift so the list feels alive.
    let mut step: i16 = 0;
    loop {
        sleep(Duration::from_millis(1400)).await;
        step = step.wrapping_add(1);
        let updates: Vec<DeviceInfo> = {
            let mut map = devices.lock().unwrap();
            map.values_mut()
                .filter(|d| d.rssi.is_some())
                .map(|d| {
                    let seed = i16::from(d.address.0[5]) + step;
                    let delta =
                        [0, 2, -3, 1, -1, 3, -2][usize::try_from(seed.rem_euclid(7)).unwrap_or(0)];
                    d.rssi = d.rssi.map(|r| (r + delta).clamp(-95, -40));
                    d.clone()
                })
                .collect()
        };
        for d in updates {
            let _ = tx.send(BtEvent::DeviceUpdated(d)).await;
        }
    }
}

fn spawn_op(
    devices: &Devices,
    tx: &mpsc::Sender<BtEvent>,
    a: Address,
    op: Operation,
    arg: Option<String>,
) {
    let devices = devices.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let error = run_op(&devices, &tx, a, op, arg).await.err();
        let _ = tx
            .send(BtEvent::OpFinished {
                address: a,
                op,
                error,
            })
            .await;
    });
}

async fn update(
    devices: &Devices,
    tx: &mpsc::Sender<BtEvent>,
    a: Address,
    f: impl FnOnce(&mut DeviceInfo),
) {
    let d = devices.lock().unwrap().get_mut(&a).map(|d| {
        f(d);
        d.clone()
    });
    if let Some(d) = d {
        let _ = tx.send(BtEvent::DeviceUpdated(d)).await;
    }
}

async fn pair(devices: &Devices, tx: &mpsc::Sender<BtEvent>, a: Address) -> Result<(), String> {
    sleep(Duration::from_millis(700)).await;
    let (reply, answer) = oneshot::channel();
    let _ = tx
        .send(BtEvent::AgentRequest(AgentRequest {
            address: a,
            kind: PromptKind::Confirm { passkey: 482_913 },
            reply,
        }))
        .await;
    match answer.await {
        Ok(AgentReply::Accept) => {}
        _ => return Err("Pairing was cancelled".into()),
    }
    sleep(Duration::from_millis(600)).await;
    update(devices, tx, a, |d| {
        d.paired = true;
        d.trusted = true;
    })
    .await;
    Ok(())
}

async fn run_op(
    devices: &Devices,
    tx: &mpsc::Sender<BtEvent>,
    a: Address,
    op: Operation,
    arg: Option<String>,
) -> Result<(), String> {
    let paired = devices.lock().unwrap().get(&a).is_some_and(|d| d.paired);
    match op {
        Operation::Connect => {
            if !paired {
                pair(devices, tx, a).await?;
            }
            sleep(Duration::from_millis(1100)).await;
            if devices
                .lock()
                .unwrap()
                .get(&a)
                .is_some_and(|d| d.rssi.is_none())
            {
                return Err("The device didn't respond. Make sure it's on and nearby".into());
            }
            update(devices, tx, a, |d| {
                d.connected = true;
                if d.battery.is_none() && d.icon.as_deref() == Some("audio-card") {
                    d.battery = Some(92);
                }
            })
            .await;
        }
        Operation::Pair => pair(devices, tx, a).await?,
        Operation::Disconnect => {
            sleep(Duration::from_millis(600)).await;
            update(devices, tx, a, |d| d.connected = false).await;
        }
        Operation::Trust => {
            sleep(Duration::from_millis(250)).await;
            update(devices, tx, a, |d| d.trusted = !d.trusted).await;
        }
        Operation::Rename => {
            sleep(Duration::from_millis(250)).await;
            update(devices, tx, a, |d| d.alias = arg.unwrap_or_default()).await;
        }
        Operation::Remove => {
            sleep(Duration::from_millis(500)).await;
            devices.lock().unwrap().remove(&a);
            let _ = tx.send(BtEvent::DeviceRemoved(a)).await;
        }
    }
    Ok(())
}
