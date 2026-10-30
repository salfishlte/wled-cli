pub mod app;
pub mod ui;

use app::{App, Focus};
use crate::api::types::DeviceCapability;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{stdout, Result};
use std::time::Duration;

pub fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;
    let mut app = App::new();

    let res = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

fn run_loop<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut need_redraw = true;

    while !app.should_quit {
        if need_redraw {
            terminal.draw(|f| ui::draw(f, app))?;
            need_redraw = false;
        }

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    need_redraw = true;

                    if app.adding_device.is_some() {
                        app.handle_modal_key(key);
                        continue;
                    }

                    if app.settings.is_some() {
                        app.handle_settings_key(key);
                        continue;
                    }

                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
                        KeyCode::Char('m') => app.cycle_current_mode(),
                        KeyCode::Char('a') => app.open_add_device(),
                        KeyCode::Char('s') => app.open_settings(),
                        KeyCode::Tab => app.cycle_focus(),
                        KeyCode::BackTab => app.cycle_focus_back(),
                        KeyCode::Char(' ') => app.toggle_power(),
                        KeyCode::Char('d') | KeyCode::Delete if app.focus == Focus::Devices => app.delete_current_device(),
                        _ => handle_focused_key(app, key.code),
                    }
                }
                Event::Resize(_, _) => need_redraw = true,
                _ => {}
            }
        }
    }
    Ok(())
}

fn handle_focused_key(app: &mut App, code: KeyCode) {
    let cap = app.current_device().map(|d| d.capability).unwrap_or(DeviceCapability::Full);

    if cap == DeviceCapability::ColdWarm {
        match code {
            KeyCode::Char('1') => return app.set_preset_cct(0, "warm"),
            KeyCode::Char('2') => return app.set_preset_cct(128, "neutral"),
            KeyCode::Char('3') => return app.set_preset_cct(255, "cold"),
            _ => {}
        }
    }

    match app.focus {
        Focus::Devices => match code {
            KeyCode::Down | KeyCode::Char('j') => app.next_device(),
            KeyCode::Up | KeyCode::Char('k') => app.prev_device(),
            KeyCode::Left | KeyCode::Char('h') if cap != DeviceCapability::OnOff => app.adjust_brightness(-5),
            KeyCode::Char('l') if cap != DeviceCapability::OnOff => app.adjust_brightness(5),
            KeyCode::Right | KeyCode::Enter if cap != DeviceCapability::OnOff => app.focus = Focus::Brightness,
            _ => {}
        },
        Focus::Brightness => match code {
            KeyCode::Left | KeyCode::Char('h') => app.adjust_brightness(-5),
            KeyCode::Right | KeyCode::Char('l') => app.adjust_brightness(5),
            KeyCode::Down | KeyCode::Char('j') => {
                app.focus = match cap {
                    DeviceCapability::ColdWarm => Focus::Cct,
                    DeviceCapability::Full => Focus::Palette,
                    DeviceCapability::OnOff => Focus::Devices,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => app.focus = Focus::Devices,
            _ => {}
        },
        Focus::Cct => match code {
            KeyCode::Left | KeyCode::Char('h') => app.adjust_cct(-13),
            KeyCode::Right | KeyCode::Char('l') => app.adjust_cct(13),
            KeyCode::Up | KeyCode::Char('k') => app.focus = Focus::Brightness,
            KeyCode::Down | KeyCode::Char('j') => app.focus = Focus::Devices,
            _ => {}
        },
        Focus::Palette => match code {
            KeyCode::Left | KeyCode::Char('h') => {
                if !app.selected_palette.is_multiple_of(3) {
                    app.selected_palette -= 1;
                } else {
                    app.focus = Focus::Devices;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if app.selected_palette % 3 < 2 && app.selected_palette + 1 < app.palette.len() {
                    app.selected_palette += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.selected_palette >= 3 {
                    app.selected_palette -= 3;
                } else {
                    app.focus = Focus::Brightness;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.selected_palette + 3 < app.palette.len() {
                    app.selected_palette += 3;
                }
            }
            KeyCode::Enter => app.apply_palette_color(),
            _ => {}
        },
    }
}
