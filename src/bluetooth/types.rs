//! Channel message types for communication between the UI thread and the
//! Bluetooth worker task. All types are plain data — no `bluer` handles cross
//! the channel boundary, keeping the TUI thread free of D-Bus concerns.

use std::fmt;

/// Re-export bluer's Address so callers don't need a direct bluer dependency.
pub use bluer::Address;
use tokio::sync::oneshot;

// ─── UI → Worker commands ────────────────────────────────────────────────────

/// Commands sent from the TUI main loop to the async Bluetooth worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BtCommand {
    /// Power the default adapter on.
    EnableAdapter,
    /// Power the default adapter off.
    DisableAdapter,
    /// Begin active device discovery (powers the adapter on if needed).
    StartScan,
    /// Stop active device discovery.
    StopScan,
    /// Full lifecycle: pair (if needed) → trust (if configured) → connect.
    Connect(Address),
    /// Graceful disconnect.
    Disconnect(Address),
    /// Initiate pairing only.
    Pair(Address),
    /// Toggle the trusted flag on a device.
    ToggleTrust(Address),
    /// Remove (forget) a cached or paired device.
    RemoveDevice(Address),
    /// Re-read every property of a single device.
    RefreshDevice(Address),
    /// Set a custom alias (friendly name) on a device.
    SetAlias(Address, String),
}

/// A long-running per-device operation. The worker runs each one as its own
/// task and always answers with exactly one [`BtEvent::OpFinished`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operation {
    Connect,
    Disconnect,
    Pair,
    Trust,
    Remove,
    Rename,
}

impl Operation {
    /// Present-progressive label shown next to the spinner.
    pub fn progress_label(self) -> &'static str {
        match self {
            Self::Connect => "Connecting",
            Self::Disconnect => "Disconnecting",
            Self::Pair => "Pairing",
            Self::Trust => "Updating trust",
            Self::Remove => "Forgetting",
            Self::Rename => "Renaming",
        }
    }

    /// Infinitive verb used in failure messages ("Couldn't connect …").
    pub fn verb(self) -> &'static str {
        match self {
            Self::Connect => "connect to",
            Self::Disconnect => "disconnect",
            Self::Pair => "pair with",
            Self::Trust => "change trust for",
            Self::Remove => "forget",
            Self::Rename => "rename",
        }
    }
}

// ─── Pairing agent prompts ──────────────────────────────────────────────────

/// What BlueZ is asking the user to decide during pairing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    /// Does this 6-digit passkey match the one shown on the device?
    Confirm { passkey: u32 },
    /// An incoming pairing request with no code ("just works").
    Authorize,
    /// A paired-but-untrusted device wants to use a service.
    AuthorizeService { service: String },
    /// The device needs a legacy PIN code typed in (often 0000 or 1234).
    EnterPin,
    /// The device needs a 6-digit passkey typed in.
    EnterPasskey,
}

/// The user's answer to a [`PromptKind`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentReply {
    Accept,
    Reject,
    /// Text typed for `EnterPin` / `EnterPasskey`.
    Text(String),
}

/// A pairing question waiting on the user. Dropping `reply` (or letting the
/// agent's timeout elapse) rejects the request.
#[derive(Debug)]
pub struct AgentRequest {
    pub address: Address,
    pub kind: PromptKind,
    pub reply: oneshot::Sender<AgentReply>,
}

// ─── Worker → UI events ─────────────────────────────────────────────────────

/// Events emitted by the Bluetooth worker back to the TUI.
#[derive(Debug)]
pub enum BtEvent {
    /// The worker connected to BlueZ and is accepting commands.
    Ready,
    /// BlueZ or the adapter is unavailable. The worker keeps retrying and
    /// sends `Ready` once things come back. The message is user-facing.
    Unavailable(String),
    /// Full adapter state snapshot.
    AdapterState(AdapterInfo),
    /// A device appeared or one of its properties changed.
    DeviceUpdated(DeviceInfo),
    /// A device was removed from BlueZ.
    DeviceRemoved(Address),
    /// A per-device operation finished. `error` is a user-facing message.
    OpFinished {
        address: Address,
        op: Operation,
        error: Option<String>,
    },
    /// The device wants the user to type `code` on it (display-only).
    PairingCode { address: Address, code: String },
    /// BlueZ needs a decision from the user.
    AgentRequest(AgentRequest),
    /// Scanning state changed.
    ScanningChanged(bool),
    /// A non-fatal, user-facing error.
    Error(String),
}

// ─── Snapshot structs ───────────────────────────────────────────────────────

/// Plain-data snapshot of the host Bluetooth adapter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdapterInfo {
    pub name: String,
    pub alias: String,
    pub address: Option<Address>,
    pub powered: bool,
    pub discovering: bool,
}

/// Plain-data snapshot of a remote Bluetooth device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub address: Address,
    /// Name the device advertises (absent for many BLE beacons).
    pub name: Option<String>,
    /// BlueZ alias: the user-set name, else `name`, else the address.
    pub alias: String,
    /// freedesktop icon name, e.g. `audio-headset`.
    pub icon: Option<String>,
    pub rssi: Option<i16>,
    pub battery: Option<u8>,
    pub paired: bool,
    pub trusted: bool,
    pub connected: bool,
    pub class: Option<u32>,
}

impl DeviceInfo {
    /// A device with only an address; the rest is filled in as known.
    pub fn new(address: Address) -> Self {
        Self {
            address,
            name: None,
            alias: address.to_string(),
            icon: None,
            rssi: None,
            battery: None,
            paired: false,
            trusted: false,
            connected: false,
            class: None,
        }
    }

    /// The name to show. BlueZ's alias already falls back to the advertised
    /// name, and reflects renames.
    pub fn display_name(&self) -> &str {
        if self.alias.is_empty() {
            self.name.as_deref().unwrap_or("Unknown device")
        } else {
            &self.alias
        }
    }

    /// No advertised name and no user alias: BlueZ fell back to the address.
    pub fn is_unnamed(&self) -> bool {
        self.name.is_none()
            && (self.alias.is_empty() || self.alias == self.address.to_string().replace(':', "-"))
    }

    /// List label: the name, or "Unnamed device · 95:0E" for anonymous beacons.
    pub fn label(&self) -> String {
        if self.is_unnamed() {
            let addr = self.address.to_string();
            format!("Unnamed device · {}", &addr[addr.len().saturating_sub(5)..])
        } else {
            self.display_name().to_string()
        }
    }

    /// Whether the user has a relationship with this device ("My devices").
    pub fn is_known(&self) -> bool {
        self.paired || self.trusted || self.connected
    }

    /// Effective sort key: connected first, then paired/trusted, then the rest.
    /// Within each tier, strongest signal first, then name for stability.
    /// Unnamed beacons sink below named devices.
    pub fn sort_key(&self) -> (u8, i16, String) {
        let tier = if self.connected {
            0
        } else if self.is_known() {
            1
        } else if self.is_unnamed() {
            3
        } else {
            2
        };
        let rssi = self.rssi.unwrap_or(i16::MIN + 1);
        (
            tier,
            rssi.saturating_neg(),
            self.display_name().to_lowercase(),
        )
    }
}

impl fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.display_name(), self.address)
    }
}
