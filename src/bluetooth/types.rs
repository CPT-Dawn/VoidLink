//! Channel message types for communication between the UI thread and the
//! Bluetooth worker task. All types are plain data — no `bluer` handles cross
//! the channel boundary, keeping the TUI thread free of D-Bus concerns.

use std::fmt;

/// Re-export bluer's Address so callers don't need a direct bluer dependency.
pub use bluer::Address;

// ─── UI → Worker commands ────────────────────────────────────────────────────

/// Commands sent from the TUI main loop to the async Bluetooth worker.
#[derive(Debug, Clone)]
pub enum BtCommand {
    /// Power the default adapter on.
    EnableAdapter,
    /// Power the default adapter off.
    DisableAdapter,
    /// Begin active device discovery.
    StartScan,
    /// Stop active device discovery.
    StopScan,
    /// Full lifecycle: pair → trust → connect.
    Connect(Address),
    /// Graceful disconnect.
    Disconnect(Address),
    /// Initiate pairing only.
    Pair(Address),
    /// Toggle the trusted flag on a device.
    Trust(Address),
    /// Remove a cached/paired device.
    RemoveDevice(Address),
    /// Re-snapshot a single device's properties.
    RefreshDevice(Address),
    /// Set a custom alias (friendly name) on a device.
    SetAlias(Address, String),
}

// ─── Worker → UI events ─────────────────────────────────────────────────────

/// Events emitted by the Bluetooth worker back to the TUI.
#[derive(Debug, Clone)]
pub enum BtEvent {
    /// Full adapter state snapshot.
    AdapterState(AdapterInfo),
    /// A new or updated device was discovered / properties changed.
    DeviceFound(DeviceInfo),
    /// A device's properties were updated (same struct, fresh snapshot).
    DeviceUpdated(DeviceInfo),
    /// A device was removed from the BlueZ object manager.
    DeviceRemoved(Address),
    /// Result of a connect attempt.
    ConnectionResult {
        address: Address,
        success: bool,
        error: Option<String>,
    },
    /// Result of a pairing attempt.
    PairResult {
        #[allow(dead_code)]
        address: Address,
        success: bool,
        error: Option<String>,
    },
    /// BlueZ is requesting the user confirm/view a PIN.
    PinRequest { address: Address, pin: String },
    /// Scanning state changed.
    ScanningChanged(bool),
    /// Catch-all error surfaced from BlueZ / D-Bus.
    Error(String),
}

// ─── Snapshot structs ───────────────────────────────────────────────────────

/// Plain-data snapshot of the host Bluetooth adapter.
#[derive(Debug, Clone, Default)]
pub struct AdapterInfo {
    pub name: String,
    pub address: Option<Address>,
    pub powered: bool,
    #[allow(dead_code)]
    pub discovering: bool,
    #[allow(dead_code)]
    pub discoverable: bool,
}

/// Plain-data snapshot of a remote Bluetooth device.
/// Created by reading all properties from a `bluer::Device` exactly once
/// and sending the result over the channel — no D-Bus handles leak out.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub address: Address,
    pub name: Option<String>,
    pub alias: String,
    pub icon: Option<String>,
    pub rssi: Option<i16>,
    #[allow(dead_code)]
    pub tx_power: Option<i16>,
    pub battery: Option<u8>,
    pub paired: bool,
    pub trusted: bool,
    pub connected: bool,
    pub class: Option<u32>,
}

impl DeviceInfo {
    /// Returns the best display name available for this device.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.alias)
    }

    /// Effective sort key: connected first, then paired/trusted, then the rest.
    /// Within each tier, sort by RSSI descending (strongest signal first).
    /// Devices with no RSSI sink to the bottom of their tier.
    pub fn sort_key(&self) -> (u8, i16) {
        let tier = if self.connected {
            0 // highest priority
        } else if self.paired || self.trusted {
            1 // known devices
        } else {
            2 // unknown / unpaired
        };
        let rssi = self.rssi.unwrap_or(i16::MIN + 1);
        // Negate RSSI so that higher (closer to 0) values sort first.
        // We use MIN + 1 above to avoid overflow when negating.
        (tier, rssi.saturating_neg())
    }
}

impl fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.display_name(), self.address)
    }
}
