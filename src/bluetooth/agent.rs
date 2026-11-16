//! BlueZ pairing agent that asks the user.
//!
//! Every pairing decision (passkey confirmation, incoming pairing, service
//! authorization, PIN/passkey entry) is forwarded to the TUI as a
//! [`BtEvent::AgentRequest`] carrying a one-shot reply channel. Nothing is
//! accepted on the user's behalf: no answer within [`PROMPT_TIMEOUT`], a
//! closed UI, or an invalid entry all reject the request.

use std::time::Duration;

use bluer::agent::{
    Agent, AgentHandle, AuthorizeService, DisplayPasskey, DisplayPinCode, ReqError, ReqResult,
    RequestAuthorization, RequestConfirmation, RequestPasskey, RequestPinCode,
};
use bluer::Address;
use tokio::sync::{mpsc, oneshot};
use tracing::info;

use super::types::{AgentReply, AgentRequest, BtEvent, PromptKind};

/// How long the user has to answer a prompt. BlueZ's own D-Bus timeout is
/// 25 s for most requests; we stay just under so the UI closes the prompt
/// itself instead of answering into the void.
pub const PROMPT_TIMEOUT: Duration = Duration::from_secs(24);

/// Ask the UI and wait for an answer.
async fn ask(
    tx: &mpsc::Sender<BtEvent>,
    address: Address,
    kind: PromptKind,
) -> ReqResult<AgentReply> {
    let (reply, answer) = oneshot::channel();
    tx.send(BtEvent::AgentRequest(AgentRequest {
        address,
        kind,
        reply,
    }))
    .await
    .map_err(|_| ReqError::Rejected)?;
    match tokio::time::timeout(PROMPT_TIMEOUT, answer).await {
        Ok(Ok(reply)) => Ok(reply),
        Ok(Err(_)) => Err(ReqError::Rejected),
        Err(_) => Err(ReqError::Canceled),
    }
}

async fn confirm(tx: &mpsc::Sender<BtEvent>, address: Address, kind: PromptKind) -> ReqResult<()> {
    match ask(tx, address, kind).await? {
        AgentReply::Accept => Ok(()),
        _ => Err(ReqError::Rejected),
    }
}

/// Parse a typed passkey: 1–6 digits, value ≤ 999999.
pub fn parse_passkey(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() || text.len() > 6 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Validate a typed legacy PIN: 1–16 printable ASCII characters.
pub fn parse_pin(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty() && text.len() <= 16 && text.bytes().all(|b| b.is_ascii_graphic()))
        .then(|| text.to_string())
}

/// Register the agent as the default BlueZ agent. The returned handle must be
/// kept alive for the agent to stay registered.
pub async fn register(
    session: &bluer::Session,
    evt_tx: mpsc::Sender<BtEvent>,
) -> bluer::Result<AgentHandle> {
    let (tx1, tx2, tx3, tx4, tx5, tx6) = (
        evt_tx.clone(),
        evt_tx.clone(),
        evt_tx.clone(),
        evt_tx.clone(),
        evt_tx.clone(),
        evt_tx.clone(),
    );

    let agent = Agent {
        request_default: true,

        request_confirmation: Some(Box::new(move |req: RequestConfirmation| {
            let tx = tx1.clone();
            Box::pin(async move {
                info!("Passkey confirmation requested by {}", req.device);
                confirm(
                    &tx,
                    req.device,
                    PromptKind::Confirm {
                        passkey: req.passkey,
                    },
                )
                .await
            })
        })),

        request_authorization: Some(Box::new(move |req: RequestAuthorization| {
            let tx = tx2.clone();
            Box::pin(async move {
                info!("Pairing authorization requested by {}", req.device);
                confirm(&tx, req.device, PromptKind::Authorize).await
            })
        })),

        authorize_service: Some(Box::new(move |req: AuthorizeService| {
            let tx = tx3.clone();
            Box::pin(async move {
                info!(
                    "Service {} authorization requested by {}",
                    req.service, req.device
                );
                let service = req.service.to_string();
                confirm(&tx, req.device, PromptKind::AuthorizeService { service }).await
            })
        })),

        request_pin_code: Some(Box::new(move |req: RequestPinCode| {
            let tx = tx4.clone();
            Box::pin(async move {
                info!("PIN code requested by {}", req.device);
                match ask(&tx, req.device, PromptKind::EnterPin).await? {
                    AgentReply::Text(text) => parse_pin(&text).ok_or(ReqError::Rejected),
                    _ => Err(ReqError::Rejected),
                }
            })
        })),

        request_passkey: Some(Box::new(move |req: RequestPasskey| {
            let tx = tx5.clone();
            Box::pin(async move {
                info!("Passkey requested by {}", req.device);
                match ask(&tx, req.device, PromptKind::EnterPasskey).await? {
                    AgentReply::Text(text) => parse_passkey(&text).ok_or(ReqError::Rejected),
                    _ => Err(ReqError::Rejected),
                }
            })
        })),

        display_passkey: Some(Box::new(move |req: DisplayPasskey| {
            let tx = tx6.clone();
            Box::pin(async move {
                let code = format!("{:06}", req.passkey);
                let _ = tx
                    .send(BtEvent::PairingCode {
                        address: req.device,
                        code,
                    })
                    .await;
                Ok(())
            })
        })),

        display_pin_code: Some(Box::new(move |req: DisplayPinCode| {
            let tx = evt_tx.clone();
            Box::pin(async move {
                let _ = tx
                    .send(BtEvent::PairingCode {
                        address: req.device,
                        code: req.pincode.clone(),
                    })
                    .await;
                Ok(())
            })
        })),

        ..Default::default()
    };

    session.register_agent(agent).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passkeys_must_be_up_to_six_digits() {
        assert_eq!(parse_passkey("123456"), Some(123_456));
        assert_eq!(parse_passkey(" 042 "), Some(42));
        assert_eq!(parse_passkey("1234567"), None);
        assert_eq!(parse_passkey("12a"), None);
        assert_eq!(parse_passkey(""), None);
        assert_eq!(parse_passkey("-1"), None);
    }

    #[test]
    fn pins_are_short_printable_ascii() {
        assert_eq!(parse_pin("0000").as_deref(), Some("0000"));
        assert_eq!(parse_pin("  abc1 ").as_deref(), Some("abc1"));
        assert_eq!(parse_pin(""), None);
        assert_eq!(parse_pin("12345678901234567"), None);
        assert_eq!(parse_pin("12 34"), None);
    }
}
