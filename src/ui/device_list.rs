//! Scrollable device list sorted by RSSI with Nerd Font icons.

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_devices();

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|device| {
            let icon = theme::device_icon(device.icon.as_deref(), device.class);
            let (rssi_icon, rssi_color) = theme::rssi_display(device.rssi);
            let name = device.display_name();

            // Build status badges.
            let mut badges = Vec::new();
            if device.connected {
                badges.push(Span::styled(" 󰂱 ", theme::connected()));
            }
            if device.paired {
                badges.push(Span::styled(" 󰌾 ", theme::paired()));
            }
            if device.trusted {
                badges.push(Span::styled(" 󰊕 ", theme::trusted()));
            }

            // Battery indicator.
            let battery_span = if let Some(pct) = device.battery {
                let (bat_icon, bat_color) = theme::battery_display(Some(pct));
                Span::styled(
                    format!(" {bat_icon} {pct}%"),
                    Style::default().fg(bat_color),
                )
            } else {
                Span::raw("")
            };

            // RSSI display.
            let rssi_span = Span::styled(format!(" {rssi_icon} "), Style::default().fg(rssi_color));

            let rssi_val = match device.rssi {
                Some(r) => Span::styled(format!("{r}dBm "), Style::default().fg(rssi_color)),
                None => Span::styled("     ", theme::dim()),
            };

            // Compose the line.
            let mut spans = vec![
                Span::styled(format!(" {icon} "), theme::list_item()),
                Span::styled(
                    format!("{name:<28}"),
                    if device.connected {
                        theme::connected()
                    } else if device.paired {
                        theme::paired()
                    } else {
                        theme::list_item()
                    },
                ),
            ];
            spans.extend(badges);
            spans.push(battery_span);
            spans.push(rssi_span);
            spans.push(rssi_val);

            ListItem::new(Line::from(spans))
        })
        .collect();

    let title = if app.scanning {
        let spinner = theme::spinner_frame(app.tick_count);
        format!(" {spinner} Devices ")
    } else {
        " Devices ".to_string()
    };

    let block = Block::default()
        .title(Span::styled(title, theme::title()))
        .borders(Borders::ALL)
        .border_style(theme::border_active());

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::selected())
        .highlight_symbol("▸ ");

    let mut state = ListState::default();
    if !filtered.is_empty() {
        state.select(Some(app.selected_index));
    }

    frame.render_stateful_widget(list, area, &mut state);
}
