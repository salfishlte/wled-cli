use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeviceCapability {
    #[serde(alias = "on_off", alias = "on_off_only", alias = "relay")]
    OnOff,
    #[serde(alias = "cold_warm", alias = "coldwarm", alias = "cct", alias = "dimmable_only", alias = "dimmable")]
    ColdWarm,
    #[serde(alias = "full", alias = "rgb")]
    Full,
}

impl DeviceCapability {
    pub fn name(&self) -> &'static str {
        match self {
            Self::OnOff => "on-off",
            Self::ColdWarm => "cold/warm",
            Self::Full => "rgb",
        }
    }

    #[allow(dead_code)]
    pub fn tag(&self) -> &'static str {
        match self {
            Self::OnOff => "pwr",
            Self::ColdWarm => "cct",
            Self::Full => "rgb",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Full => Self::ColdWarm,
            Self::ColdWarm => Self::OnOff,
            Self::OnOff => Self::Full,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct WledWifiInfo {
    pub bssid: Option<String>,
    pub signal: Option<i32>,
    pub channel: Option<u8>,
    pub rssi: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct WledInfo {
    pub ver: Option<String>,
    pub vid: Option<u32>,
    pub name: String,
    pub arch: Option<String>,
    pub uptime: Option<u64>,
    pub freeheap: Option<u32>,
    pub mac: Option<String>,
    pub ip: Option<String>,
    pub wifi: Option<WledWifiInfo>,
    pub leds: Option<LedsInfo>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct LedsInfo {
    pub count: Option<u16>,
    pub pwr: Option<u32>,
    pub maxpwr: Option<u32>,
    pub fps: Option<u8>,
    pub rgb: Option<bool>,
    pub rgbw: Option<bool>,
    pub wv: Option<serde_json::Value>,
    pub cct: Option<serde_json::Value>,
    pub lc: Option<u8>,
    pub seglc: Option<Vec<u8>>,
}

impl WledInfo {
    pub fn capability(&self) -> DeviceCapability {
        let leds = match &self.leds {
            Some(l) => l,
            None => return DeviceCapability::Full,
        };

        let has_cct = match &leds.cct {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::Number(n)) => n.as_u64().is_some_and(|v| v > 0),
            _ => false,
        };

        if let Some(lc) = leds.lc {
            if (lc & 1) != 0 { return DeviceCapability::Full; }
            if (lc & 6) != 0 || has_cct { return DeviceCapability::ColdWarm; }
            if lc == 0 { return DeviceCapability::OnOff; }
        }

        if let Some(seglc) = &leds.seglc {
            if seglc.iter().any(|&b| (b & 1) != 0) { return DeviceCapability::Full; }
            if seglc.iter().any(|&b| (b & 6) != 0) || has_cct { return DeviceCapability::ColdWarm; }
            if seglc.iter().all(|&b| b == 0) && leds.count.unwrap_or(0) <= 1 { return DeviceCapability::OnOff; }
        }

        if has_cct { return DeviceCapability::ColdWarm; }
        if leds.rgb == Some(false) {
            return if leds.count.unwrap_or(0) <= 1 { DeviceCapability::OnOff } else { DeviceCapability::ColdWarm };
        }
        if leds.count.unwrap_or(1) <= 1 && leds.rgb.is_none() {
            return DeviceCapability::OnOff;
        }

        DeviceCapability::Full
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Segment {
    pub cct: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WledState {
    pub on: Option<bool>,
    pub bri: Option<u8>,
    pub seg: Option<Vec<Segment>>,
}

impl WledState {
    pub fn cct(&self) -> u8 {
        self.seg.as_ref().and_then(|s| s.first()).and_then(|s| s.cct).map(|c| c.min(255) as u8).unwrap_or(128)
    }

    pub fn set_cct(&mut self, val: u8) {
        if let Some(first) = self.seg.as_mut().and_then(|s| s.first_mut()) {
            first.cct = Some(val as u16);
        } else {
            self.seg = Some(vec![Segment { cct: Some(val as u16) }]);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteColor {
    pub name: &'static str,
    pub rgb: [u8; 3],
}

const fn pc(name: &'static str, rgb: [u8; 3]) -> PaletteColor {
    PaletteColor { name, rgb }
}

pub const BASE_PALETTE: &[PaletteColor] = &[
    pc("red", [255, 0, 0]), pc("green", [0, 255, 0]), pc("blue", [0, 0, 255]),
    pc("cyan", [0, 255, 255]), pc("magenta", [255, 0, 255]), pc("yellow", [255, 255, 0]),
    pc("white", [255, 255, 255]), pc("warm-white", [255, 180, 107]), pc("cold-white", [201, 226, 255]),
    pc("orange", [255, 136, 0]), pc("amber", [255, 191, 0]), pc("coral", [255, 127, 80]),
    pc("flame", [226, 88, 34]), pc("sunset", [253, 94, 83]), pc("peach", [255, 176, 124]),
    pc("crimson", [220, 20, 60]), pc("scarlet", [255, 36, 0]), pc("ruby", [224, 17, 95]),
    pc("pink", [255, 105, 180]), pc("hot-pink", [255, 20, 147]), pc("rose", [255, 0, 127]),
    pc("purple", [155, 48, 255]), pc("violet", [238, 130, 238]), pc("indigo", [75, 0, 130]),
    pc("lavender", [181, 126, 220]), pc("plum", [221, 160, 221]), pc("orchid", [218, 112, 214]),
    pc("sky-blue", [135, 206, 235]), pc("deep-sky", [0, 191, 255]), pc("dodger-blue", [30, 144, 255]),
    pc("royal-blue", [65, 105, 225]), pc("ice-blue", [160, 230, 255]), pc("ocean", [0, 119, 190]),
    pc("aqua", [0, 255, 255]), pc("turquoise", [64, 224, 208]), pc("teal", [0, 128, 128]),
    pc("lime", [0, 255, 0]), pc("lime-green", [50, 205, 50]), pc("emerald", [80, 200, 120]),
    pc("mint", [152, 255, 152]), pc("spring-green", [0, 255, 127]), pc("aurora", [120, 224, 143]),
    pc("gold", [255, 215, 0]), pc("lemon", [255, 244, 79]), pc("canary", [255, 239, 0]),
    pc("neon-red", [255, 7, 58]), pc("neon-green", [57, 255, 20]), pc("neon-blue", [77, 77, 255]),
    pc("neon-pink", [255, 110, 199]), pc("neon-purple", [188, 19, 254]), pc("neon-cyan", [0, 245, 255]),
    pc("pastel-pink", [255, 182, 193]), pc("pastel-blue", [174, 198, 207]), pc("pastel-yellow", [253, 253, 150]),
    pc("pastel-green", [119, 221, 119]), pc("pastel-purple", [195, 177, 225]), pc("pastel-orange", [255, 179, 71]),
    pc("vapor-cyan", [5, 255, 161]), pc("vapor-pink", [255, 113, 206]), pc("synth-violet", [185, 103, 255]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_capability_cycle() {
        assert_eq!(DeviceCapability::Full.next(), DeviceCapability::ColdWarm);
        assert_eq!(DeviceCapability::ColdWarm.next(), DeviceCapability::OnOff);
        assert_eq!(DeviceCapability::OnOff.next(), DeviceCapability::Full);
    }

    #[test]
    fn test_device_capability_tags() {
        assert_eq!(DeviceCapability::Full.tag(), "rgb");
        assert_eq!(DeviceCapability::ColdWarm.tag(), "cct");
        assert_eq!(DeviceCapability::OnOff.tag(), "pwr");
    }

    #[test]
    fn test_cct_state() {
        let mut state = WledState::default();
        assert_eq!(state.cct(), 128);
        state.set_cct(200);
        assert_eq!(state.cct(), 200);
    }

    #[test]
    fn test_wled_pc_capability_detection() {
        let json = r#"{"name":"wled-pc","leds":{"count":70,"pwr":120,"fps":0,"maxpwr":850,"maxseg":32,"bootps":0,"seglc":[1],"lc":1,"rgbw":false,"wv":0,"cct":0}}"#;
        let info: WledInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.capability(), DeviceCapability::Full);
    }

    #[test]
    fn test_cct_garland_capability_detection() {
        let json = r#"{"name":"wled-bed","leds":{"count":50,"seglc":[4],"lc":4,"rgbw":false,"wv":0,"cct":1}}"#;
        let info: WledInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.capability(), DeviceCapability::ColdWarm);
    }

    #[test]
    fn test_relay_onoff_capability_detection() {
        let json = r#"{"name":"relay","leds":{"count":1,"seglc":[0],"lc":0,"rgbw":false,"wv":0,"cct":0}}"#;
        let info: WledInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.capability(), DeviceCapability::OnOff);
    }
}
