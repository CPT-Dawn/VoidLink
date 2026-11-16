//! Toast notifications and modal dialogs (pairing prompts, confirmations).

use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap};
use ratatui::Frame;

use super::{modal_frame, wrapped_height};
use crate::app::{App, Modal, Prompt, ToastKind};
use crate::bluetooth::types::PromptKind;
use crate::theme;

// ─── Toasts ─────────────────────────────────────────────────────────────────

/// Stack toasts in the bottom-right corner of `area`, newest at the bottom.
pub fn render_toasts(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(4).min(54);
    if width < 20 {
        return;
    }
    // Sit just inside the panels' bottom border.
    let mut bottom = area.y + area.height.saturating_sub(1);
    for toast in app.toasts.iter().rev() {
        let (icon, color) = match toast.kind {
            ToastKind::Success => (theme::ok_icon(), theme::green()),
            ToastKind::Error => (theme::err_icon(), theme::red()),
            ToastKind::Info => (theme::info_icon(), theme::accent()),
        };
        let text = vec![Line::from(vec![
            Span::styled(format!("{icon}  "), theme::bold(color)),
            Span::styled(toast.message.clone(), theme::body()),
        ])];
        let height = wrapped_height(&text, width.saturating_sub(4)).min(4) + 2;
        if bottom < area.y + height {
            break;
        }
        bottom -= height;

        // Slide in from the right.
        let eased = theme::ease_out_cubic(toast.progress());
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let offset = ((1.0 - eased) * f32::from(width)) as u16;
        let x = area.x + area.width - width - 2 + offset;
        let rect = Rect {
            x,
            y: bottom,
            width: width.min(area.x + area.width - x),
            height,
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(theme::fg(color))
            .padding(Padding::horizontal(1));
        frame.render_widget(Clear, rect);
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: true }).block(block),
            rect,
        );
    }
}

// ─── Modals ─────────────────────────────────────────────────────────────────

pub fn render_modal(frame: &mut Frame, app: &App) {
    match &app.modal {
        None => {}
        Some(Modal::Help) => super::help::render(frame),
        Some(Modal::Prompt(p)) => render_prompt(frame, app, p),
        Some(Modal::PairingCode { address, code }) => {
            let name = app.name_of(*address);
            let lines = vec![
                Line::from(Span::styled(
                    format!("Type this code on {name}:"),
                    theme::body(),
                )),
                Line::raw(""),
                big_code(code),
                Line::raw(""),
                Line::from(Span::styled(
                    "then press Enter on the device.",
                    theme::dim(),
                )),
            ];
            dialog(
                frame,
                &format!(" {} Pairing ", theme::lock_icon()),
                lines,
                theme::accent(),
            );
        }
        Some(Modal::ConfirmRemove(address)) => {
            let name = app.name_of(*address);
            let lines = vec![
                Line::from(vec![
                    Span::styled("Forget ", theme::body()),
                    Span::styled(name, theme::bold(theme::text())),
                    Span::styled("?", theme::body()),
                ]),
                Line::raw(""),
                Line::from(Span::styled(
                    "You'll need to pair it again before you can use it.",
                    theme::dim(),
                )),
                Line::raw(""),
                choices("Forget", "Cancel"),
            ];
            dialog(frame, " Forget device ", lines, theme::red());
        }
    }
}

fn render_prompt(frame: &mut Frame, app: &App, p: &Prompt) {
    let name = app.name_of(p.address);
    let who = || Span::styled(name.clone(), theme::bold(theme::text()));
    let mut lines = match &p.kind {
        PromptKind::Confirm { passkey } => vec![
            Line::from(vec![who(), Span::styled(" wants to pair.", theme::body())]),
            Line::from(Span::styled(
                "Does this code match the one on the device?",
                theme::body(),
            )),
            Line::raw(""),
            big_code(&format!("{passkey:06}")),
            Line::raw(""),
            choices("Pair", "Cancel"),
        ],
        PromptKind::Authorize => vec![
            Line::from(vec![
                who(),
                Span::styled(" wants to pair with this computer.", theme::body()),
            ]),
            Line::raw(""),
            Line::from(Span::styled(
                "Only accept if you started pairing on the device.",
                theme::dim(),
            )),
            Line::raw(""),
            choices("Allow", "Deny"),
        ],
        PromptKind::AuthorizeService { service } => vec![
            Line::from(vec![
                who(),
                Span::styled(
                    format!(" wants to use {}.", service_name(service)),
                    theme::body(),
                ),
            ]),
            Line::raw(""),
            choices("Allow", "Deny"),
        ],
        PromptKind::EnterPin | PromptKind::EnterPasskey => {
            let (what, tip) = if p.kind == PromptKind::EnterPasskey {
                ("passkey", "The 6-digit number shown on the device")
            } else {
                ("PIN", "Often 0000 or 1234 — check the device's manual")
            };
            let mut l = vec![
                Line::from(vec![
                    Span::styled(format!("Enter the {what} for "), theme::body()),
                    who(),
                ]),
                Line::from(Span::styled(tip, theme::dim())),
                Line::raw(""),
                Line::from(vec![
                    Span::styled("› ", theme::bold(theme::accent2())),
                    Span::styled(p.input.clone(), theme::bold(theme::text())),
                    Span::styled("▏", theme::bold(theme::accent())),
                ]),
            ];
            if let Some(err) = &p.error {
                l.push(Line::from(Span::styled(err.clone(), theme::error())));
            }
            l
        }
    };
    lines.push(Line::raw(""));
    lines.push(
        Line::from(Span::styled(
            format!("Expires in {}s", p.seconds_left()),
            theme::dim(),
        ))
        .right_aligned(),
    );
    dialog(
        frame,
        &format!(" {} Pairing request ", theme::lock_icon()),
        lines,
        theme::accent(),
    );
}

fn dialog(frame: &mut Frame, title: &str, lines: Vec<Line<'static>>, color: ratatui::style::Color) {
    let width = 58u16.min(frame.area().width.saturating_sub(4));
    let height = wrapped_height(&lines, width.saturating_sub(6)) + 3;
    let inner = modal_frame(frame, title, width, height, color);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

/// A pairing code, spaced out and centred: "4 8 2   9 1 3".
fn big_code(code: &str) -> Line<'static> {
    let mut spaced = String::new();
    for (i, c) in code.chars().enumerate() {
        if i > 0 {
            spaced.push_str(if i == code.len() / 2 && code.len() == 6 {
                "   "
            } else {
                " "
            });
        }
        spaced.push(c);
    }
    Line::from(Span::styled(spaced, theme::bold(theme::accent()))).centered()
}

fn choices(yes: &str, no: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled("y", theme::key()),
        Span::styled(format!(" {yes}     "), theme::body()),
        Span::styled("n", theme::key()),
        Span::styled(format!(" {no}"), theme::body()),
    ])
}

/// Friendly names for the services that commonly ask for authorization.
fn service_name(uuid: &str) -> String {
    let short = uuid.get(4..8).unwrap_or("").to_ascii_lowercase();
    let name = match short.as_str() {
        "110a" | "110b" | "110d" => "audio (A2DP)",
        "110c" | "110e" | "110f" => "media controls (AVRCP)",
        "1108" | "1112" => "headset audio (HSP)",
        "111e" | "111f" => "hands-free calling (HFP)",
        "1124" => "input (keyboard/mouse)",
        "1105" => "file transfer (OPP)",
        "1106" => "file transfer (FTP)",
        "1115" | "1116" => "network access (PAN)",
        "112f" => "phonebook access",
        "1132" => "message access",
        _ => return format!("service {uuid}"),
    };
    name.to_string()
}
