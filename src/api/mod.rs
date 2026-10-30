pub mod types;

use std::time::Duration;
use serde::Serialize;
use types::{WledInfo, WledState};

#[derive(Serialize)]
struct TogglePayload<'a> { on: &'a str }

#[derive(Serialize)]
struct PowerPayload { on: bool }

#[derive(Serialize)]
struct BriPayload { on: bool, bri: u8 }

#[derive(Serialize)]
struct CctSeg { cct: u8 }

#[derive(Serialize)]
struct CctPayload { on: bool, seg: [CctSeg; 1] }

#[derive(Serialize)]
struct ColorSeg { col: [[u8; 3]; 1], fx: u8 }

#[derive(Serialize)]
struct ColorPayload { on: bool, seg: [ColorSeg; 1] }

#[inline]
fn url_endpoint(ip: &str, ep: &str) -> String {
    let mut url = String::with_capacity(7 + ip.len() + 6 + ep.len());
    url.push_str("http://");
    url.push_str(ip);
    url.push_str("/json/");
    url.push_str(ep);
    url
}

#[derive(Clone)]
pub struct WledClient {
    agent: ureq::Agent,
}

impl Default for WledClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WledClient {
    pub fn new() -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_millis(250))
            .timeout_read(Duration::from_millis(600))
            .timeout_write(Duration::from_millis(600))
            .max_idle_connections_per_host(4)
            .build();
        Self { agent }
    }

    fn post_state<T: Serialize>(&self, ip: &str, body: &T) -> Result<(), String> {
        self.agent
            .post(&url_endpoint(ip, "state"))
            .send_json(body)
            .map_err(|e| format!("api error {}: {}", ip, e))?;
        Ok(())
    }

    pub fn get_info(&self, ip: &str) -> Result<WledInfo, String> {
        self.agent
            .get(&url_endpoint(ip, "info"))
            .call()
            .map_err(|e| format!("connection error to {}: {}", ip, e))?
            .into_json::<WledInfo>()
            .map_err(|e| format!("parse info error from {}: {}", ip, e))
    }

    pub fn get_state(&self, ip: &str) -> Result<WledState, String> {
        self.agent
            .get(&url_endpoint(ip, "state"))
            .call()
            .map_err(|e| format!("connection error to {}: {}", ip, e))?
            .into_json::<WledState>()
            .map_err(|e| format!("parse state error from {}: {}", ip, e))
    }

    pub fn toggle(&self, ip: &str) -> Result<bool, String> {
        let res: WledState = self.agent
            .post(&url_endpoint(ip, "state"))
            .send_json(&TogglePayload { on: "t" })
            .map_err(|e| format!("connection error to {}: {}", ip, e))?
            .into_json()
            .map_err(|e| format!("parse state error from {}: {}", ip, e))?;
        Ok(res.on.unwrap_or(false))
    }

    #[allow(dead_code)]
    pub fn set_power(&self, ip: &str, on: bool) -> Result<(), String> {
        self.post_state(ip, &PowerPayload { on })
    }

    pub fn set_bri(&self, ip: &str, bri: u8) -> Result<(), String> {
        self.post_state(ip, &BriPayload { on: true, bri })
    }

    pub fn set_cct(&self, ip: &str, cct: u8) -> Result<(), String> {
        self.post_state(ip, &CctPayload { on: true, seg: [CctSeg { cct }] })
    }

    pub fn set_color(&self, ip: &str, rgb: [u8; 3]) -> Result<(), String> {
        self.post_state(ip, &ColorPayload { on: true, seg: [ColorSeg { col: [rgb], fx: 0 }] })
    }

    pub fn reboot(&self, ip: &str) -> Result<(), String> {
        let _ = self.agent
            .post(&url_endpoint(ip, "state"))
            .send_json(&serde_json::json!({ "rb": true }));
        Ok(())
    }

    pub fn post_cfg<T: Serialize>(&self, ip: &str, body: &T) -> Result<(), String> {
        self.agent
            .post(&url_endpoint(ip, "cfg"))
            .send_json(body)
            .map_err(|e| format!("cfg error {}: {}", ip, e))?;
        Ok(())
    }

    pub fn set_mdns(&self, ip: &str, mdns: &str) -> Result<(), String> {
        self.post_cfg(ip, &serde_json::json!({ "id": { "mdns": mdns } }))
    }

    pub fn set_wifi(&self, ip: &str, ssid: &str, pswd: &str) -> Result<(), String> {
        self.post_cfg(ip, &serde_json::json!({ "nw": { "ins": [{ "ssid": ssid, "pswd": pswd }] } }))
    }

    pub fn set_power_limit(&self, ip: &str, maxpwr: u32) -> Result<(), String> {
        self.post_cfg(ip, &serde_json::json!({ "hw": { "led": { "maxpwr": maxpwr } } }))
    }

    pub fn set_boot_on(&self, ip: &str, on: bool) -> Result<(), String> {
        self.post_cfg(ip, &serde_json::json!({ "def": { "on": on } }))
    }
}
