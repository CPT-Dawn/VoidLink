//! Turn BlueZ / D-Bus errors into messages a user can act on.

use bluer::{Error, ErrorKind};

/// Hint shown whenever BlueZ itself is missing or stopped.
pub const START_SERVICE_HINT: &str = "Start it with: sudo systemctl enable --now bluetooth.service";

/// A short, actionable description of a failed operation.
pub fn humanize(err: &Error) -> String {
    let msg = err.message.as_str();
    let lower = msg.to_lowercase();

    if lower.contains("rfkill") || lower.contains("blocked") {
        return "Bluetooth is blocked by rfkill. Unblock it with: rfkill unblock bluetooth".into();
    }
    if lower.contains("profile-unavailable") || lower.contains("protocol not available") {
        return "No audio profile is available. Install pipewire-pulse (or pulseaudio-bluetooth) and try again"
            .into();
    }
    if lower.contains("page-timeout") || lower.contains("host is down") {
        return "The device didn't respond. Make sure it's on, nearby and in pairing mode".into();
    }
    if lower.contains("serviceunknown") || lower.contains("was not provided by any .service") {
        return format!("The Bluetooth service (bluetoothd) isn't running. {START_SERVICE_HINT}");
    }

    match &err.kind {
        ErrorKind::AuthenticationCanceled => "Pairing was cancelled".into(),
        ErrorKind::AuthenticationFailed | ErrorKind::AuthenticationRejected => {
            "Pairing was rejected. Check the code on the device and try again".into()
        }
        ErrorKind::AuthenticationTimeout => {
            "Pairing timed out. Put the device in pairing mode and try again".into()
        }
        ErrorKind::ConnectionAttemptFailed => {
            "The device didn't respond. Make sure it's on and nearby".into()
        }
        ErrorKind::AlreadyConnected => "Already connected".into(),
        ErrorKind::AlreadyExists => "Already paired".into(),
        ErrorKind::InProgress => "Another operation is already running for this device".into(),
        ErrorKind::NotReady => "The adapter isn't ready. Is Bluetooth powered on?".into(),
        ErrorKind::DoesNotExist | ErrorKind::NotFound => {
            "BlueZ no longer knows this device. Scan again to rediscover it".into()
        }
        ErrorKind::NotAuthorized | ErrorKind::NotPermitted => {
            "Permission denied by BlueZ. Check your D-Bus/polkit policy".into()
        }
        ErrorKind::NotSupported => "This device doesn't support that".into(),
        _ if msg.is_empty() => format!("{:?}", err.kind),
        _ => msg.trim_end_matches('.').to_string(),
    }
}

/// Message shown when the worker can't reach BlueZ or find an adapter.
pub fn unavailable(err: &Error) -> String {
    let lower = err.message.to_lowercase();
    if lower.contains("serviceunknown") || lower.contains("not provided by any") {
        return format!("The Bluetooth service isn't running. {START_SERVICE_HINT}");
    }
    if lower.contains("dbus") && (lower.contains("connect") || lower.contains("socket")) {
        return "Can't reach the system D-Bus. Is dbus running?".into();
    }
    match err.kind {
        ErrorKind::NotFound => {
            "No Bluetooth adapter found. Plug one in, or check: rfkill list bluetooth".into()
        }
        _ => humanize(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(kind: ErrorKind, message: &str) -> Error {
        Error {
            kind,
            message: message.into(),
        }
    }

    #[test]
    fn explains_missing_audio_stack() {
        let e = err(ErrorKind::Failed, "br-connection-profile-unavailable");
        assert!(humanize(&e).contains("pipewire-pulse"));
    }

    #[test]
    fn explains_rfkill() {
        let e = err(ErrorKind::Failed, "Blocked through rfkill");
        assert!(humanize(&e).contains("rfkill unblock"));
    }

    #[test]
    fn explains_stopped_daemon() {
        let e = err(
            ErrorKind::Failed,
            "org.freedesktop.DBus.Error.ServiceUnknown: The name org.bluez was not provided by any .service files",
        );
        assert!(unavailable(&e).contains("systemctl enable --now bluetooth"));
    }

    #[test]
    fn missing_adapter() {
        let e = err(ErrorKind::NotFound, "");
        assert!(unavailable(&e).contains("No Bluetooth adapter"));
    }

    #[test]
    fn falls_back_to_bluez_message() {
        let e = err(ErrorKind::Failed, "Something odd happened.");
        assert_eq!(humanize(&e), "Something odd happened");
    }
}
