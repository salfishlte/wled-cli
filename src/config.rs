use crate::api::types::DeviceCapability;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceEntry {
    pub ip: String,
    pub name: String,
    pub capability: DeviceCapability,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AppConfig {
    pub default_device: Option<String>,
    pub devices: BTreeMap<String, DeviceEntry>,
}

impl AppConfig {
    pub fn file_path() -> Result<PathBuf, String> {
        if let Ok(custom_dir) = std::env::var("WLED_CONFIG_DIR") {
            return Ok(PathBuf::from(custom_dir).join("config.json"));
        }
        let proj_dirs = ProjectDirs::from("", "", "wled-cli")
            .ok_or_else(|| "failed to resolve config directory".to_string())?;
        Ok(proj_dirs.config_dir().join("config.json"))
    }

    pub fn load() -> Result<Option<Self>, String> {
        let path = Self::file_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("failed to read config at {}: {}", path.display(), e))?;
        let config: AppConfig = serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse config at {}: {}", path.display(), e))?;
        Ok(Some(config))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create dir {}: {}", parent.display(), e))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize config: {}", e))?;
        fs::write(&path, content)
            .map_err(|e| format!("failed to write config to {}: {}", path.display(), e))?;
        Ok(())
    }
}
