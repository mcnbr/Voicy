use log::info;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::app_state::Settings as AppSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub source_language: String,
    pub target_language: String,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub operation_mode: crate::app_state::OperationMode,
    pub live_pause_threshold_ms: u64,
    pub volume: f32,
    pub auto_play: bool,
    pub models_path: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            source_language: "auto".to_string(),
            target_language: "en".to_string(),
            input_device: None,
            output_device: None,
            operation_mode: crate::app_state::OperationMode::Automatic,
            live_pause_threshold_ms: 1500,
            volume: 1.0,
            auto_play: true,
            models_path: None,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("voicy")
        .join("config.json")
}

pub fn load_settings() -> Settings {
    let path = get_config_path();
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(settings) => {
                    info!("Settings loaded from {:?}", path);
                    return settings;
                }
                Err(e) => {
                    log::warn!("Failed to parse settings: {}", e);
                }
            },
            Err(e) => {
                log::warn!("Failed to read settings: {}", e);
            }
        }
    }
    info!("Using default settings");
    Settings::default()
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let path = get_config_path();

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())?;

    info!("Settings saved to {:?}", path);
    Ok(())
}