use crate::api::types::DeviceCapability;
use crate::tui::app::{AddDeviceModal, AddField, App, Focus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();
    f.render_widget(Clear, size);
    f.render_widget(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" wled-cli "), size);

    let inner = Rect {
        x: size.x.saturating_add(2),
        y: size.y.saturating_add(1),
        width: size.width.saturating_sub(4),
        height: size.height.saturating_sub(2),
    };

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(1)])
        .split(inner);

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(35)])
        .split(main_chunks[0]);

    draw_devices(f, app, content_chunks[0]);
    draw_controls(f, app, content_chunks[1]);
    draw_footer(f, app, main_chunks[1]);

    if let Some(modal) = &app.adding_device {
        draw_add_device_modal(f, modal, size);
    }
}

fn draw_devices(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == Focus::Devices;
    let border_style = Style::default().fg(if is_focused { Color::Cyan } else { Color::DarkGray });

    let items: Vec<ListItem> = if app.devices.is_empty() {
        vec![
            ListItem::new(Line::from(Span::styled("  (no devices)", Style::default().fg(Color::DarkGray)))),
            ListItem::new(Line::from(Span::styled("  press 'a' to add", Style::default().fg(Color::DarkGray)))),
        ]
    } else {
        let name_max = (area.width.saturating_sub(9) as usize).max(4);
        app.devices.iter().enumerate().map(|(idx, (_, dev))| {
            let is_sel = idx == app.selected_device;
            let cursor = if is_sel { "> " } else { "  " };
            let (dot, dot_col) = if is_sel {
                if app.state.as_ref().and_then(|s| s.on).unwrap_or(false) { ("●", Color::Green) } else { ("○", Color::DarkGray) }
            } else {
                ("○", Color::DarkGray)
            };
            let name = if dev.name.len() > name_max { &dev.name[..name_max] } else { &dev.name };
            let pad = " ".repeat(name_max.saturating_sub(name.len()));

            ListItem::new(Line::from(vec![
                Span::styled(cursor, if is_sel { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default() }),
                Span::styled(format!("{}. ", idx + 1), Style::default().fg(Color::DarkGray)),
                Span::styled(name.to_lowercase(), if is_sel { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) }),
                Span::raw(pad),
                Span::styled(format!(" {}", dot), Style::default().fg(dot_col)),
            ]))
        }).collect()
    };

    let title_style = if is_focused { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) };
    let list = List::new(items).block(Block::default().borders(Borders::ALL).border_style(border_style).title(Span::styled(" devices ", title_style)));
    f.render_widget(list, area);
}

fn draw_controls(f: &mut Frame, app: &App, area: Rect) {
    let dev = match app.current_device() {
        Some(d) => d,
        None => {
            let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)).title(" controls ");
            f.render_widget(Paragraph::new(" no device selected. press 'a' to add.").block(block), area);
            return;
        }
    };

    let ctrl_focused = app.focus == Focus::Brightness;
    let border_color = if ctrl_focused { Color::Cyan } else { Color::DarkGray };
    let dev_title = format!(" current: {} ({}) [{}] ", dev.name, dev.ip, dev.capability.name());
    let is_on = app.state.as_ref().and_then(|s| s.on).unwrap_or(false);
    let power_line = Line::from(vec![
        Span::raw(" power: "),
        Span::styled(if is_on { "[ on ]" } else { "[ off ]" }, Style::default().fg(if is_on { Color::Green } else { Color::DarkGray }).add_modifier(Modifier::BOLD)),
    ]);

    match dev.capability {
        DeviceCapability::OnOff => {
            let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color)).title(dev_title);
            let inner = block.inner(area);
            f.render_widget(block, area);
            let text = vec![
                power_line,
                Line::raw(""),
                Line::from(vec![Span::styled(" mode: ", Style::default().fg(Color::DarkGray)), Span::styled("on/off (relay / switch)", Style::default().fg(Color::Yellow))]),
                Line::raw(""),
                Line::from(Span::styled(" note: brightness and colors are disabled for this device", Style::default().fg(Color::DarkGray))),
            ];
            f.render_widget(Paragraph::new(text), inner);
        }
        DeviceCapability::ColdWarm => {
            let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(if ctrl_focused || app.focus == Focus::Cct { Color::Cyan } else { Color::DarkGray })).title(dev_title);
            let inner = block.inner(area);
            f.render_widget(block, area);

            let bri_raw = app.state.as_ref().and_then(|s| s.bri).unwrap_or(0);
            let bri_pct = ((bri_raw as f32 / 255.0) * 100.0).round() as u16;
            let cct_raw = app.state.as_ref().map(|s| s.cct()).unwrap_or(128);
            let cct_pct = ((cct_raw as f32 / 255.0) * 100.0).round() as u16;
            let kelvin = 2700 + ((cct_raw as u32 * 3800) / 255);
            let bar_w = (inner.width.saturating_sub(26) as usize).clamp(10, 20);

            let text = vec![
                power_line,
                Line::raw(""),
                build_bar(" brightness:  ", bri_pct, &format!("{:>3}%", bri_pct), bar_w, app.focus == Focus::Brightness, Color::Cyan),
                Line::raw(""),
                build_bar(" temperature: ", cct_pct, &format!("{}k ({:>3}%)", kelvin, cct_pct), bar_w, app.focus == Focus::Cct, Color::Yellow),
                Line::raw(""),
                Line::from(vec![
                    Span::styled(" presets:     ", Style::default().fg(Color::DarkGray)),
                    Span::styled("[1: warm 2700k] ", Style::default().fg(Color::Yellow)),
                    Span::styled("[2: neutral 4200k] ", Style::default().fg(Color::White)),
                    Span::styled("[3: cold 6500k]", Style::default().fg(Color::Cyan)),
                ]),
                Line::raw(""),
                Line::from(vec![Span::styled(" mode:        ", Style::default().fg(Color::DarkGray)), Span::styled("cold/warm tunable white (no rgb)", Style::default().fg(Color::DarkGray))]),
            ];
            f.render_widget(Paragraph::new(text), inner);
        }
        DeviceCapability::Full => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(5), Constraint::Min(6)])
                .split(area);

            let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(border_color)).title(dev_title);
            let inner = block.inner(chunks[0]);
            f.render_widget(block, chunks[0]);

            let bri_raw = app.state.as_ref().and_then(|s| s.bri).unwrap_or(0);
            let bri_pct = ((bri_raw as f32 / 255.0) * 100.0).round() as u16;
            let bar_w = (inner.width.saturating_sub(26) as usize).clamp(10, 20);

            let text = vec![
                power_line,
                Line::raw(""),
                build_bar(" brightness:  ", bri_pct, &format!("{:>3}%", bri_pct), bar_w, ctrl_focused, Color::Cyan),
            ];
            f.render_widget(Paragraph::new(text), inner);
            draw_palette(f, app, chunks[1]);
        }
    }
}

fn build_bar(label: &str, pct: u16, suffix: &str, bar_width: usize, is_focused: bool, bar_color: Color) -> Line<'static> {
    let filled = ((pct as usize * bar_width) / 100).min(bar_width);
    let bar_str = if filled == 0 {
        " ".repeat(bar_width)
    } else if filled == bar_width {
        "=".repeat(bar_width)
    } else {
        format!("{}>{}", "=".repeat(filled.saturating_sub(1)), " ".repeat(bar_width - filled))
    };

    Line::from(vec![
        Span::styled(label.to_string(), if is_focused { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) }),
        Span::styled("[", Style::default().fg(Color::DarkGray)),
        Span::styled(bar_str, Style::default().fg(if is_focused { bar_color } else { Color::DarkGray })),
        Span::styled("] ", Style::default().fg(Color::DarkGray)),
        Span::styled(suffix.to_string(), if is_focused { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) }),
    ])
}

fn draw_palette(f: &mut Frame, app: &App, area: Rect) {
    let pal_focused = app.focus == Focus::Palette;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if pal_focused { Color::Cyan } else { Color::DarkGray }))
        .title(Span::styled(" color palette ", if pal_focused { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) }));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let total_rows = app.palette.len().div_ceil(3);
    let selected_row = app.selected_palette / 3;
    let visible_rows = (inner.height as usize).max(1);
    let scroll_row = if selected_row >= visible_rows { selected_row - visible_rows + 1 } else { 0 };

    let render_swatch = |idx: usize, col: &crate::api::types::PaletteColor| -> [Span<'static>; 4] {
        let is_sel = app.selected_palette == idx && pal_focused;
        let c = Color::Rgb(col.rgb[0], col.rgb[1], col.rgb[2]);
        let (bl, br) = if is_sel { (" >[", "]< ") } else { ("  [", "]  ") };
        let bracket_style = if is_sel { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::DarkGray) };
        let name_style = if is_sel { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) };

        [
            Span::styled(bl, bracket_style),
            Span::styled("■ ", Style::default().fg(c)),
            Span::styled(format!("{:<14}", col.name), name_style),
            Span::styled(br, bracket_style),
        ]
    };

    for r in 0..visible_rows {
        let actual_row = scroll_row + r;
        if actual_row >= total_rows { break; }
        let row_area = Rect { x: inner.x, y: inner.y + r as u16, width: inner.width, height: 1 };
        if row_area.y >= inner.bottom() { break; }

        let mut row_spans = Vec::with_capacity(12);
        for c in 0..3 {
            let color_idx = actual_row * 3 + c;
            if let Some(col) = app.palette.get(color_idx) {
                row_spans.extend(render_swatch(color_idx, col));
            }
        }
        f.render_widget(Paragraph::new(Line::from(row_spans)), row_area);
    }
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let status_color = if app.status_msg.starts_with("error") || app.status_msg.contains("error") { Color::Red } else { Color::DarkGray };

    let footer_line = Line::from(vec![
        Span::styled(" [arrows]", Style::default().fg(Color::Cyan)),
        Span::raw(": navigate  |"),
        Span::styled(" [space]", Style::default().fg(Color::Cyan)),
        Span::raw(": power  |"),
        Span::styled(" [m]", Style::default().fg(Color::Cyan)),
        Span::raw(": mode  |"),
        Span::styled(" [enter]", Style::default().fg(Color::Cyan)),
        Span::raw(": apply  |"),
        Span::styled(" [a]", Style::default().fg(Color::Cyan)),
        Span::raw(": add  |"),
        Span::styled(" [d]", Style::default().fg(Color::Cyan)),
        Span::raw(": del  |"),
        Span::styled(" [q]", Style::default().fg(Color::Cyan)),
        Span::raw(": quit  "),
        Span::styled(format!("-> {}", app.status_msg), Style::default().fg(status_color)),
    ]);

    f.render_widget(Paragraph::new(footer_line), area);
}

fn draw_add_device_modal(f: &mut Frame, modal: &AddDeviceModal, area: Rect) {
    let w = 46.min(area.width.saturating_sub(4));
    let h = 7.min(area.height.saturating_sub(2));
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    };
    f.render_widget(Clear, popup);
    let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Cyan)).title(" add device ");
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let ip_f = modal.active_field == AddField::Ip;
    let name_f = modal.active_field == AddField::Name;
    let mode_f = modal.active_field == AddField::Mode;

    let field_line = |label: &str, val: &str, placeholder: &str, is_f: bool| -> Line<'static> {
        let (text, style) = if val.is_empty() {
            (placeholder.to_string(), Style::default().fg(Color::DarkGray))
        } else if is_f {
            (val.to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED | Modifier::BOLD))
        } else {
            (val.to_string(), Style::default().fg(Color::White))
        };
        Line::from(vec![
            Span::styled(format!(" {}: [ ", label), if is_f { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) }),
            Span::styled(format!("{:<26}", text), style),
            Span::raw(" ]"),
        ])
    };

    let mode_str = match modal.mode {
        DeviceCapability::Full => "rgb (ws2812b / full color)",
        DeviceCapability::ColdWarm => "cct (cold/warm tunable)",
        DeviceCapability::OnOff => "on/off (relay / switch)",
    };

    let lines = vec![
        field_line("ip  ", &modal.ip, "192.168.1.x", ip_f),
        field_line("name", &modal.name, "my-wled", name_f),
        Line::from(vec![
            Span::styled(" mode: [ ", if mode_f { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::Gray) }),
            Span::styled(format!("{:<26}", mode_str), if mode_f { Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) }),
            Span::raw(" ]"),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("[tab]", Style::default().fg(Color::Cyan)),
            Span::raw(":next "),
            Span::styled("[space]", Style::default().fg(Color::Cyan)),
            Span::raw(":mode "),
            Span::styled("[enter]", Style::default().fg(Color::Cyan)),
            Span::raw(":save "),
            Span::styled("[esc]", Style::default().fg(Color::Cyan)),
            Span::raw(":cancel"),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), inner);
}
