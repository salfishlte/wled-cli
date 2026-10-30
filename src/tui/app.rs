use crate::api::types::{DeviceCapability, PaletteColor, WledInfo, WledState, BASE_PALETTE};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Network,
    Power,
    About,
}

impl SettingsTab {
    pub fn next(&self) -> Self {
        match self {
            Self::Network => Self::Power,
            Self::Power => Self::About,
            Self::About => Self::Network,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Network => Self::About,
            Self::Power => Self::Network,
            Self::About => Self::Power,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkField {
    Mdns,
    WifiSsid,
    WifiPass,
    SaveWifi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerField {
    MaxPower,
    BootOn,
    SavePower,
}

pub struct SettingsModal {
    pub tab: SettingsTab,
    pub network_field: NetworkField,
    pub power_field: PowerField,
    pub mdns: String,
    pub wifi_ssid: String,
    pub wifi_pass: String,
    pub max_power: String,
    pub boot_on: bool,
    pub status: String,
    pub confirm_reboot: bool,
}

pub struct App {
    pub client: WledClient,
    pub devices: Vec<(String, DeviceEntry)>,
    pub selected_device: usize,
    pub state: Option<WledState>,
    pub info: Option<WledInfo>,
    pub focus: Focus,
    pub palette: &'static [PaletteColor],
    pub selected_palette: usize,
    pub status_msg: String,
    pub should_quit: bool,
    pub adding_device: Option<AddDeviceModal>,
    pub settings: Option<SettingsModal>,
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
            info: None,
            focus: Focus::Devices,
            palette: BASE_PALETTE,
            selected_palette: 0,
            status_msg: String::from("ready"),
            should_quit: false,
            adding_device: None,
            settings: None,
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
            if let Ok(info) = self.client.get_info(&ip) {
                self.info = Some(info);
            }
            match self.client.get_state(&ip) {
                Ok(state) => {
                    self.state = Some(state);
                    self.status_msg = format!("connected to {}", name);
                }
                Err(e) => {
                    self.info = None;
                    self.status_msg = format!("error: {}", e);
                }
            }
        } else {
            self.state = None;
            self.info = None;
            self.status_msg = String::from("no devices configured");
        }
    }

    pub fn open_settings(&mut self) {
        if let Some(dev) = self.current_device() {
            let ip = dev.ip.clone();
            let info = self.client.get_info(&ip).ok();
            let max_pwr_str = info.as_ref()
                .and_then(|i| i.leds.as_ref())
                .and_then(|l| l.maxpwr)
                .map(|p| p.to_string())
                .unwrap_or_else(|| "850".to_string());
            let mdns_init = dev.name.clone();

            self.settings = Some(SettingsModal {
                tab: SettingsTab::Network,
                network_field: NetworkField::Mdns,
                power_field: PowerField::MaxPower,
                mdns: mdns_init,
                wifi_ssid: String::new(),
                wifi_pass: String::new(),
                max_power: max_pwr_str,
                boot_on: true,
                status: String::from("[tab]: switch tabs  [enter]: save/action"),
                confirm_reboot: false,
            });
            if info.is_some() {
                self.info = info;
            }
        } else {
            self.status_msg = String::from("no device selected for settings");
        }
    }

    pub fn handle_settings_key(&mut self, key: KeyEvent) {
        let Some(settings) = &mut self.settings else { return; };
        let Some(dev) = self.devices.get(self.selected_device).map(|(_, d)| d.clone()) else { return; };
        let ip = dev.ip.clone();

        match key.code {
            KeyCode::Esc => {
                self.settings = None;
                return;
            }
            KeyCode::Tab | KeyCode::Right => {
                settings.tab = settings.tab.next();
                settings.confirm_reboot = false;
                return;
            }
            KeyCode::BackTab | KeyCode::Left => {
                settings.tab = settings.tab.prev();
                settings.confirm_reboot = false;
                return;
            }
            KeyCode::Char('1') if settings.network_field == NetworkField::SaveWifi || settings.power_field == PowerField::SavePower || settings.power_field == PowerField::BootOn || settings.tab == SettingsTab::About => {
                settings.tab = SettingsTab::Network;
                settings.confirm_reboot = false;
                return;
            }
            KeyCode::Char('2') if settings.network_field == NetworkField::SaveWifi || settings.power_field == PowerField::SavePower || settings.power_field == PowerField::BootOn || settings.tab == SettingsTab::About => {
                settings.tab = SettingsTab::Power;
                settings.confirm_reboot = false;
                return;
            }
            KeyCode::Char('3') if settings.network_field == NetworkField::SaveWifi || settings.power_field == PowerField::SavePower || settings.power_field == PowerField::BootOn || settings.tab == SettingsTab::About => {
                settings.tab = SettingsTab::About;
                settings.confirm_reboot = false;
                return;
            }
            _ => {}
        }

        match settings.tab {
            SettingsTab::Network => match key.code {
                KeyCode::Up => {
                    settings.network_field = match settings.network_field {
                        NetworkField::Mdns => NetworkField::SaveWifi,
                        NetworkField::WifiSsid => NetworkField::Mdns,
                        NetworkField::WifiPass => NetworkField::WifiSsid,
                        NetworkField::SaveWifi => NetworkField::WifiPass,
                    };
                }
                KeyCode::Down => {
                    settings.network_field = match settings.network_field {
                        NetworkField::Mdns => NetworkField::WifiSsid,
                        NetworkField::WifiSsid => NetworkField::WifiPass,
                        NetworkField::WifiPass => NetworkField::SaveWifi,
                        NetworkField::SaveWifi => NetworkField::Mdns,
                    };
                }
                KeyCode::Backspace => match settings.network_field {
                    NetworkField::Mdns => { settings.mdns.pop(); }
                    NetworkField::WifiSsid => { settings.wifi_ssid.pop(); }
                    NetworkField::WifiPass => { settings.wifi_pass.pop(); }
                    _ => {}
                },
                KeyCode::Enter => match settings.network_field {
                    NetworkField::Mdns => {
                        let name = settings.mdns.trim();
                        if !name.is_empty() {
                            match self.client.set_mdns(&ip, name) {
                                Ok(_) => settings.status = format!("mdns updated to '{}.local'", name),
                                Err(e) => settings.status = format!("error: {}", e),
                            }
                        }
                    }
                    NetworkField::WifiSsid => {
                        settings.network_field = NetworkField::WifiPass;
                    }
                    NetworkField::WifiPass | NetworkField::SaveWifi => {
                        let ssid = settings.wifi_ssid.trim();
                        let pass = settings.wifi_pass.trim();
                        if ssid.is_empty() {
                            settings.status = String::from("error: ssid cannot be empty");
                        } else {
                            match self.client.set_wifi(&ip, ssid, pass) {
                                Ok(_) => settings.status = format!("wifi credentials saved for '{}'", ssid),
                                Err(e) => settings.status = format!("error: {}", e),
                            }
                        }
                    }
                },
                KeyCode::Char(c) => match settings.network_field {
                    NetworkField::Mdns if settings.mdns.len() < 32 && (c.is_ascii_alphanumeric() || c == '-' || c == '_') => {
                        settings.mdns.push(c.to_ascii_lowercase());
                    }
                    NetworkField::WifiSsid if settings.wifi_ssid.len() < 32 && !c.is_ascii_control() => {
                        settings.wifi_ssid.push(c);
                    }
                    NetworkField::WifiPass if settings.wifi_pass.len() < 64 && !c.is_ascii_control() => {
                        settings.wifi_pass.push(c);
                    }
                    _ => {}
                },
                _ => {}
            },
            SettingsTab::Power => match key.code {
                KeyCode::Up => {
                    settings.power_field = match settings.power_field {
                        PowerField::MaxPower => PowerField::SavePower,
                        PowerField::BootOn => PowerField::MaxPower,
                        PowerField::SavePower => PowerField::BootOn,
                    };
                }
                KeyCode::Down => {
                    settings.power_field = match settings.power_field {
                        PowerField::MaxPower => PowerField::BootOn,
                        PowerField::BootOn => PowerField::SavePower,
                        PowerField::SavePower => PowerField::MaxPower,
                    };
                }
                KeyCode::Char(' ') if settings.power_field == PowerField::BootOn => {
                    settings.boot_on = !settings.boot_on;
                }
                KeyCode::Backspace if settings.power_field == PowerField::MaxPower => {
                    settings.max_power.pop();
                }
                KeyCode::Char(c) if settings.power_field == PowerField::MaxPower && c.is_ascii_digit() && settings.max_power.len() < 6 => {
                    settings.max_power.push(c);
                }
                KeyCode::Enter => match settings.power_field {
                    PowerField::BootOn => {
                        settings.boot_on = !settings.boot_on;
                    }
                    PowerField::MaxPower | PowerField::SavePower => {
                        if let Ok(ma) = settings.max_power.trim().parse::<u32>() {
                            let r1 = self.client.set_power_limit(&ip, ma);
                            let r2 = self.client.set_boot_on(&ip, settings.boot_on);
                            if r1.is_ok() && r2.is_ok() {
                                settings.status = format!("saved: limit {} ma, boot {}", ma, if settings.boot_on { "on" } else { "off" });
                            } else {
                                settings.status = String::from("error saving power settings");
                            }
                        } else {
                            settings.status = String::from("error: invalid ma number");
                        }
                    }
                },
                _ => {}
            },
            SettingsTab::About => match key.code {
                KeyCode::Enter => {
                    if !settings.confirm_reboot {
                        settings.confirm_reboot = true;
                        settings.status = String::from("⚠️ press enter to confirm reboot");
                    } else {
                        let _ = self.client.reboot(&ip);
                        settings.confirm_reboot = false;
                        settings.status = String::from("reboot signal sent to device");
                        self.status_msg = format!("rebooting {}", dev.name.to_lowercase());
                    }
                }
                _ => {
                    settings.confirm_reboot = false;
                }
            },
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
