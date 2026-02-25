//! Custom BlueZ agent for handling pairing PIN/passkey requests.
//!
//! When a device requires user confirmation (e.g. "Confirm passkey 123456"),
//! the default BlueZ agent cannot interact with a TUI. This agent forwards
//! pairing prompts to the UI via the `BtEvent` channel.

use bluer::agent::{
    Agent, AgentHandle, DisplayPasskey, DisplayPinCode, RequestAuthorization, RequestConfirmation,
    RequestPasskey, RequestPinCode,
};
use tokio::sync::mpsc;
use tracing::info;

use super::types::BtEvent;

/// Register our custom agent with the BlueZ session. Returns a handle that
/// must be kept alive for the agent to remain registered.
pub async fn register(
    session: &bluer::Session,
    evt_tx: mpsc::Sender<BtEvent>,
) -> bluer::Result<AgentHandle> {
    let evt_tx_confirm = evt_tx.clone();
    let evt_tx_display = evt_tx.clone();

    let agent = Agent {
        request_default: true,

        request_confirmation: Some(Box::new(move |req: RequestConfirmation| {
            let tx = evt_tx_confirm.clone();
            Box::pin(async move {
                let pin = format!("{:06}", req.passkey);
                info!(
                    "Pairing confirmation request from {}: passkey {}",
                    req.device, pin
                );
                let _ = tx
                    .send(BtEvent::PinRequest {
                        address: req.device,
                        pin,
                    })
                    .await;
                // Auto-confirm — in a more sophisticated implementation
                // we'd wait for user confirmation via a return channel.
                Ok(())
            })
        })),

        display_passkey: Some(Box::new(move |req: DisplayPasskey| {
            let tx = evt_tx_display.clone();
            Box::pin(async move {
                let pin = format!("{:06}", req.passkey);
                info!("Display passkey for {}: {}", req.device, pin);
                let _ = tx
                    .send(BtEvent::PinRequest {
                        address: req.device,
                        pin,
                    })
                    .await;
                Ok(())
            })
        })),

        request_passkey: Some(Box::new(|req: RequestPasskey| {
            Box::pin(async move {
                // For devices that need a numeric passkey, return a default.
                // A full implementation would prompt the user.
                info!("Passkey requested for {}", req.device);
                Ok(0)
            })
        })),

        display_pin_code: Some(Box::new(move |req: DisplayPinCode| {
            let tx = evt_tx.clone();
            Box::pin(async move {
                info!("Display PIN for {}: {}", req.device, req.pincode);
                let _ = tx
                    .send(BtEvent::PinRequest {
                        address: req.device,
                        pin: req.pincode.clone(),
                    })
                    .await;
                Ok(())
            })
        })),

        request_pin_code: Some(Box::new(|req: RequestPinCode| {
            Box::pin(async move {
                info!("PIN code requested for {}", req.device);
                Ok("0000".into())
            })
        })),

        request_authorization: Some(Box::new(|req: RequestAuthorization| {
            Box::pin(async move {
                info!("Authorization requested for {}", req.device);
                Ok(())
            })
        })),

        ..Default::default()
    };

    session.register_agent(agent).await
}
