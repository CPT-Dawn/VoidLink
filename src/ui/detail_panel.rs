//! Detail panel for the currently selected device.

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(" Details ", theme::title()))
        .borders(Borders::ALL)
        .border_style(theme::border_inactive());

    let Some(device) = app.selected_device() else {
        // No device selected — show placeholder.
        let placeholder = Paragraph::new(Line::from(vec![Span::styled(
            "  Select a device to view details",
            theme::dim(),
        )]))
        .block(block);
        frame.render_widget(placeholder, area);
        return;
    };

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner area into lines.
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // name
            Constraint::Length(1), // address
            Constraint::Length(1), // spacer
            Constraint::Length(1), // status badges
            Constraint::Length(1), // spacer
            Constraint::Length(1), // RSSI label
            Constraint::Length(1), // RSSI gauge
            Constraint::Length(1), // spacer
            Constraint::Length(1), // battery label
            Constraint::Length(1), // battery gauge
            Constraint::Length(1), // spacer
            Constraint::Length(1), // device class
            Constraint::Length(1), // icon type
            Constraint::Min(0),   // rest
        ])
        .split(inner);

    // ── Name ────────────────────────────────────────────────────────────
    let icon = theme::device_icon(device.icon.as_deref(), device.class);
    let name_line = Line::from(vec![
        Span::styled(format!("  {icon} "), theme::title()),
        Span::styled(device.display_name(), theme::title()),
    ]);
    frame.render_widget(Paragraph::new(name_line), chunks[0]);

    // ── Address ─────────────────────────────────────────────────────────
    let addr_line = Line::from(vec![
        Span::styled("  Address: ", theme::dim()),
        Span::styled(device.address.to_string(), theme::list_item()),
    ]);
    frame.render_widget(Paragraph::new(addr_line), chunks[1]);

    // ── Status badges ───────────────────────────────────────────────────
    let mut badge_spans = vec![Span::styled("  Status:  ", theme::dim())];
    if device.connected {
        badge_spans.push(Span::styled("● Connected  ", theme::connected()));
    } else {
        badge_spans.push(Span::styled("○ Disconnected  ", theme::dim()));
    }
    if device.paired {
        badge_spans.push(Span::styled("󰌾 Paired  ", theme::paired()));
    } else {
        badge_spans.push(Span::styled("  Not Paired  ", theme::dim()));
    }
    if device.trusted {
        badge_spans.push(Span::styled("󰊕 Trusted", theme::trusted()));
    } else {
        badge_spans.push(Span::styled("  Not Trusted", theme::dim()));
    }
    frame.render_widget(Paragraph::new(Line::from(badge_spans)), chunks[3]);

    // ── RSSI ────────────────────────────────────────────────────────────
    let (rssi_icon, rssi_color) = theme::rssi_display(device.rssi);
    let rssi_text = match device.rssi {
        Some(r) => format!("  {rssi_icon} Signal: {r} dBm  {}", theme::rssi_bar(device.rssi)),
        None => format!("  {rssi_icon} Signal: N/A"),
    };
    let rssi_line = Line::from(Span::styled(rssi_text, Style::default().fg(rssi_color)));
    frame.render_widget(Paragraph::new(rssi_line), chunks[5]);

    // RSSI gauge (map -100..0 dBm to 0..100%).
    if let Some(rssi) = device.rssi {
        let pct = ((rssi.max(-100) + 100) as f64 / 100.0 * 100.0).clamp(0.0, 100.0) as u16;
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(rssi_color))
            .percent(pct)
            .label(format!("{pct}%"));
        frame.render_widget(gauge, chunks[6]);
    }

    // ── Battery ─────────────────────────────────────────────────────────
    let (bat_icon, bat_color) = theme::battery_display(device.battery);
    let bat_text = match device.battery {
        Some(pct) => format!("  {bat_icon} Battery: {pct}%"),
        None => format!("  {bat_icon} Battery: N/A"),
    };
    let bat_line = Line::from(Span::styled(bat_text, Style::default().fg(bat_color)));
    frame.render_widget(Paragraph::new(bat_line), chunks[8]);

    if let Some(pct) = device.battery {
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(bat_color))
            .percent(pct as u16)
            .label(format!("{pct}%"));
        frame.render_widget(gauge, chunks[9]);
    }

    // ── Device class / icon type ────────────────────────────────────────
    if let Some(class) = device.class {
        let class_line = Line::from(vec![
            Span::styled("  Class:   ", theme::dim()),
            Span::styled(format!("0x{class:06X}"), theme::list_item()),
        ]);
        frame.render_widget(Paragraph::new(class_line), chunks[11]);
    }

    if let Some(ref icon_name) = device.icon {
        let icon_line = Line::from(vec![
            Span::styled("  Type:    ", theme::dim()),
            Span::styled(icon_name.as_str(), theme::list_item()),
        ]);
        frame.render_widget(Paragraph::new(icon_line), chunks[12]);
    }
}
