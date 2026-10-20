use crate::api::types::{DeviceCapability, PaletteColor, WledState, BASE_PALETTE};
use crate::api::WledClient;
use crate::config::{AppConfig, DeviceEntry};
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Devices,
    Brightness,
    Cct,
    Palette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddField {
    Ip,
    Name,
    Mode,
}

pub struct AddDeviceModal {
    pub ip: String,
    pub name: String,
    pub mode: DeviceCapability,
    pub active_field: AddField,
}

pub struct App {
    pub client: WledClient,
    pub devices: Vec<(String, DeviceEntry)>,
    pub selected_device: usize,
    pub state: Option<WledState>,
    pub focus: Focus,
    pub palette: &'static [PaletteColor],
    pub selected_palette: usize,
    pub status_msg: String,
    pub should_quit: bool,
    pub adding_device: Option<AddDeviceModal>,
}

impl App {
    pub fn new() -> Self {
        let client = WledClient::new();
        let cfg = AppConfig::load().ok().flatten().unwrap_or_default();
        let devices: Vec<(String, DeviceEntry)> = cfg.devices.into_iter().collect();

        let mut app = Self {
            client,
            devices,
            selected_device: 0,
            state: None,
            focus: Focus::Devices,
            palette: BASE_PALETTE,
            selected_palette: 0,
            status_msg: String::from("ready"),
            should_quit: false,
            adding_device: None,
        };

        if app.devices.is_empty() {
            app.open_add_device();
        } else {
            app.refresh_state();
        }
        app
    }

    pub fn current_device(&self) -> Option<&DeviceEntry> {
        self.devices.get(self.selected_device).map(|(_, dev)| dev)
    }

    pub fn open_add_device(&mut self) {
        self.adding_device = Some(AddDeviceModal {
            ip: String::new(),
            name: String::new(),
            mode: DeviceCapability::Full,
            active_field: AddField::Ip,
        });
    }

    pub fn modal_detect_mode(&mut self) {
        if let Some(modal) = &mut self.adding_device {
            let ip = modal.ip.trim();
            if !ip.is_empty() {
                if let Ok(info) = self.client.get_info(ip) {
                    modal.mode = info.capability();
                    if modal.name.is_empty() && !info.name.is_empty() {
                        modal.name = info.name.to_lowercase();
                    }
                }
            }
        }
    }

    pub fn handle_modal_key(&mut self, key: KeyEvent) {
        let mut detect = false;
        let mut save = false;

        if let Some(modal) = &mut self.adding_device {
            match key.code {
                KeyCode::Esc => {
                    self.adding_device = None;
                    return;
                }
                KeyCode::Tab | KeyCode::Down => {
                    modal.active_field = match modal.active_field {
                        AddField::Ip => { detect = true; AddField::Name }
                        AddField::Name => AddField::Mode,
                        AddField::Mode => AddField::Ip,
                    };
                }
                KeyCode::Up | KeyCode::BackTab => {
                    modal.active_field = match modal.active_field {
                        AddField::Ip => AddField::Mode,
                        AddField::Name => AddField::Ip,
                        AddField::Mode => AddField::Name,
                    };
                }
                KeyCode::Left | KeyCode::Right => {
                    if modal.active_field == AddField::Mode {
                        modal.mode = modal.mode.next();
                    }
                }
                KeyCode::Enter => match modal.active_field {
                    AddField::Ip if !modal.ip.is_empty() => { detect = true; modal.active_field = AddField::Name; }
                    AddField::Name => modal.active_field = AddField::Mode,
                    AddField::Mode => save = true,
                    _ => {}
                },
                KeyCode::Backspace => match modal.active_field {
                    AddField::Ip => { modal.ip.pop(); }
                    AddField::Name => { modal.name.pop(); }
                    _ => {}
                },
                KeyCode::Char(' ') => match modal.active_field {
                    AddField::Mode => modal.mode = modal.mode.next(),
                    AddField::Name if modal.name.len() < 24 => modal.name.push(' '),
                    _ => {}
                },
                KeyCode::Char(c) => match modal.active_field {
                    AddField::Ip if modal.ip.len() < 30 && (c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':') => {
                        modal.ip.push(c);
                    }
                    AddField::Name if modal.name.len() < 24 && (c.is_ascii_alphanumeric() || c == '-' || c == '_') => {
                        modal.name.push(c.to_ascii_lowercase());
                    }
                    AddField::Mode => modal.mode = match c {
                        '1' => DeviceCapability::Full,
                        '2' => DeviceCapability::ColdWarm,
                        '3' => DeviceCapability::OnOff,
                        _ => modal.mode.next(),
                    },
                    _ => {}
                },
                _ => {}
            }
        }

        if detect {
            self.modal_detect_mode();
        }
        if save {
            self.save_new_device();
        }
    }

    pub fn save_new_device(&mut self) {
        if let Some(modal) = self.adding_device.take() {
            let ip = modal.ip.trim().to_string();
            if ip.is_empty() {
                self.status_msg = String::from("error: ip cannot be empty");
                return;
            }
            let name = if modal.name.trim().is_empty() {
                format!("wled-{}", ip.split('.').next_back().unwrap_or("dev"))
            } else {
                modal.name.trim().to_lowercase()
            };

            let entry = DeviceEntry { ip: ip.clone(), name: name.clone(), capability: modal.mode };
            self.devices.push((name.clone(), entry));
            self.selected_device = self.devices.len() - 1;
            self.save_config();

            self.status_msg = format!("added device {}", name);
            self.refresh_state();
        }
    }

    pub fn save_config(&self) {
        let mut cfg = AppConfig {
            default_device: self.devices.get(self.selected_device).map(|(k, _)| k.clone()),
            ..Default::default()
        };
        for (k, v) in &self.devices {
            cfg.devices.insert(k.clone(), v.clone());
        }
        let _ = cfg.save();
    }

    pub fn delete_current_device(&mut self) {
        if !self.devices.is_empty() {
            let removed = self.devices.remove(self.selected_device);
            if self.selected_device >= self.devices.len() && !self.devices.is_empty() {
                self.selected_device = self.devices.len() - 1;
            }
            self.save_config();
            self.status_msg = format!("deleted device {}", removed.0);
            self.refresh_state();
        }
    }

    pub fn cycle_current_mode(&mut self) {
        if let Some((_, dev)) = self.devices.get_mut(self.selected_device) {
            dev.capability = dev.capability.next();
            let new_mode = dev.capability;
            let dev_name = dev.name.clone();
            self.save_config();

            if new_mode == DeviceCapability::OnOff {
                self.focus = Focus::Devices;
            } else if new_mode == DeviceCapability::ColdWarm && self.focus == Focus::Palette {
                self.focus = Focus::Brightness;
            }
            self.status_msg = format!("mode for {} set to {}", dev_name, new_mode.name());
        }
    }

    pub fn refresh_state(&mut self) {
        if let Some((_, dev)) = self.devices.get_mut(self.selected_device) {
            let (ip, name) = (dev.ip.clone(), dev.name.clone());
            match self.client.get_state(&ip) {
                Ok(state) => {
                    self.state = Some(state);
                    self.status_msg = format!("connected to {}", name);
                }
                Err(e) => self.status_msg = format!("error: {}", e),
            }
        } else {
            self.state = None;
            self.status_msg = String::from("no devices configured");
        }
    }

    pub fn toggle_power(&mut self) {
        if let Some(dev) = self.current_device() {
            let ip = dev.ip.clone();
            match self.client.toggle(&ip) {
                Ok(new_state) => {
                    if let Some(s) = &mut self.state { s.on = Some(new_state); }
                    self.status_msg = format!("power: {}", if new_state { "on" } else { "off" });
                }
                Err(e) => self.status_msg = format!("toggle error: {}", e),
            }
        }
    }

    pub fn adjust_brightness(&mut self, delta_percent: i16) {
        if let Some(dev) = self.current_device() {
            if dev.capability == DeviceCapability::OnOff {
                self.status_msg = String::from("on/off mode: no brightness control");
                return;
            }
            let ip = dev.ip.clone();
            let cur = self.state.as_ref().and_then(|s| s.bri).unwrap_or(0);
            let pct = (((cur as f32 / 255.0) * 100.0).round() as i16 + delta_percent).clamp(0, 100) as u8;
            let raw = ((pct as f32 / 100.0) * 255.0).round() as u8;

            if let Some(s) = &mut self.state {
                s.bri = Some(raw);
                s.on = Some(true);
            }
            self.status_msg = format!("brightness: {}%", pct);
            if let Err(e) = self.client.set_bri(&ip, raw) {
                self.status_msg = format!("bri error: {}", e);
            }
        }
    }

    fn apply_cct(&mut self, next_cct: u8) {
        if let Some(dev) = self.current_device() {
            if dev.capability != DeviceCapability::ColdWarm { return; }
            let ip = dev.ip.clone();
            if let Some(s) = &mut self.state {
                s.set_cct(next_cct);
                s.on = Some(true);
            }
            let kelvin = 2700 + ((next_cct as u32 * 3800) / 255);
            let pct = ((next_cct as f32 / 255.0) * 100.0).round() as u16;
            self.status_msg = format!("temperature: {}k ({}% cold)", kelvin, pct);
            if let Err(e) = self.client.set_cct(&ip, next_cct) {
                self.status_msg = format!("cct error: {}", e);
            }
        }
    }

    pub fn adjust_cct(&mut self, delta: i16) {
        let cur = self.state.as_ref().map(|s| s.cct()).unwrap_or(128);
        self.apply_cct((cur as i16 + delta).clamp(0, 255) as u8);
    }

    pub fn set_preset_cct(&mut self, cct: u8, name: &str) {
        self.apply_cct(cct);
        let kelvin = 2700 + ((cct as u32 * 3800) / 255);
        self.status_msg = format!("preset: {} ({}k)", name, kelvin);
    }

    pub fn apply_palette_color(&mut self) {
        if let Some(dev) = self.current_device() {
            if dev.capability != DeviceCapability::Full { return; }
            let ip = dev.ip.clone();
            if let Some(col) = self.palette.get(self.selected_palette) {
                let (rgb, name) = (col.rgb, col.name);
                match self.client.set_color(&ip, rgb) {
                    Ok(_) => {
                        if let Some(s) = &mut self.state { s.on = Some(true); }
                        self.status_msg = format!("color: {}", name);
                    }
                    Err(e) => self.status_msg = format!("color error: {}", e),
                }
            }
        }
    }

    pub fn cycle_focus(&mut self) {
        let cap = self.current_device().map(|d| d.capability).unwrap_or(DeviceCapability::Full);
        self.focus = match (self.focus, cap) {
            (_, DeviceCapability::OnOff) => Focus::Devices,
            (Focus::Devices, DeviceCapability::ColdWarm) => Focus::Brightness,
            (Focus::Brightness, DeviceCapability::ColdWarm) => Focus::Cct,
            (Focus::Cct, DeviceCapability::ColdWarm) => Focus::Devices,
            (Focus::Devices, DeviceCapability::Full) => Focus::Brightness,
            (Focus::Brightness, DeviceCapability::Full) => Focus::Palette,
            (Focus::Palette, DeviceCapability::Full) => Focus::Devices,
            _ => Focus::Devices,
        };
    }

    pub fn cycle_focus_back(&mut self) {
        let cap = self.current_device().map(|d| d.capability).unwrap_or(DeviceCapability::Full);
        self.focus = match (self.focus, cap) {
            (_, DeviceCapability::OnOff) => Focus::Devices,
            (Focus::Devices, DeviceCapability::ColdWarm) => Focus::Cct,
            (Focus::Cct, DeviceCapability::ColdWarm) => Focus::Brightness,
            (Focus::Brightness, DeviceCapability::ColdWarm) => Focus::Devices,
            (Focus::Devices, DeviceCapability::Full) => Focus::Palette,
            (Focus::Brightness, DeviceCapability::Full) => Focus::Devices,
            (Focus::Palette, DeviceCapability::Full) => Focus::Brightness,
            _ => Focus::Devices,
        };
    }

    pub fn next_device(&mut self) {
        if !self.devices.is_empty() {
            self.selected_device = (self.selected_device + 1) % self.devices.len();
            self.refresh_state();
        }
    }

    pub fn prev_device(&mut self) {
        if !self.devices.is_empty() {
            self.selected_device = self.selected_device.checked_sub(1).unwrap_or(self.devices.len() - 1);
            self.refresh_state();
        }
    }
}
