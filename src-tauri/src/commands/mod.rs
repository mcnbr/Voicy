use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use tauri::State;
use log::{error, info};

use crate::app_state::{AppState, HardwareInfo, OperationMode, PipelineStatus};
use crate::audio::{AudioCapture, AudioPlayback};
use crate::pipeline::Pipeline;
use crate::config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfoResponse {
    pub has_cuda: bool,
    pub cuda_device: Option<String>,
    pub vram_used_mb: u64,
    pub vram_available_mb: u64,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevicesResponse {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub current_mode: String,
    pub is_capturing: bool,
    pub whisper_time_ms: u64,
    pub translate_time_ms: u64,
    pub tts_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsResponse {
    pub source_language: String,
    pub target_language: String,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub operation_mode: String,
    pub live_pause_threshold_ms: u64,
    pub volume: f32,
    pub auto_play: bool,
}

#[tauri::command]
pub fn get_hardware_info(state: State<'_, Arc<RwLock<AppState>>>) -> Result<HardwareInfoResponse, String> {
    let state = state.read();
    Ok(HardwareInfoResponse {
        has_cuda: state.hardware.has_cuda,
        cuda_device: state.hardware.cuda_device.clone(),
        vram_used_mb: state.hardware.vram_used_mb,
        vram_available_mb: state.hardware.vram_available_mb,
        ram_used_mb: state.hardware.ram_used_mb,
        ram_total_mb: state.hardware.ram_total_mb,
    })
}

#[tauri::command]
pub fn get_audio_devices(state: State<'_, Arc<RwLock<AppState>>>) -> Result<AudioDevicesResponse, String> {
    let capture = AudioCapture::new();
    Ok(AudioDevicesResponse {
        input: capture.list_input_devices(),
        output: capture.list_output_devices(),
    })
}

#[tauri::command]
pub fn set_input_device(device: String, state: State<'_, Arc<RwLock<AppState>>>) -> Result<(), String> {
    let mut state = state.write();
    state.audio.input_device = Some(device);
    Ok(())
}

#[tauri::command]
pub fn set_output_device(device: String, state: State<'_, Arc<RwLock<AppState>>>) -> Result<(), String> {
    let mut state = state.write();
    state.audio.output_device = Some(device);
    Ok(())
}

#[tauri::command]
pub fn set_source_language(language: String, state: State<'_, Arc<RwLock<AppState>>>) -> Result<(), String> {
    let mut state = state.write();
    state.settings.source_language = language;
    Ok(())
}

#[tauri::command]
pub fn set_target_language(language: String, state: State<'_, Arc<RwLock<AppState>>>) -> Result<(), String> {
    let mut state = state.write();
    state.settings.target_language = language;
    Ok(())
}

#[tauri::command]
pub async fn start_capture(state: State<'_, Arc<RwLock<AppState>>>) -> Result<(), String> {
    let mut state = state.write();
    if state.pipeline.status != PipelineStatus::Idle {
        return Err("Already capturing".to_string());
    }
    state.pipeline.status = PipelineStatus::Capturing;
    state.audio.is_capturing = true;
    info!("Capture started");
    Ok(())
}

#[tauri::command]
pub async fn stop_capture(state: State<'_, Arc<RwLock<AppState>>>) -> Result<(), String> {
    let mut state = state.write();
    state.audio.is_capturing = false;
    state.pipeline.status = PipelineStatus::Idle;
    info!("Capture stopped");
    Ok(())
}

#[tauri::command]
pub fn get_status(state: State<'_, Arc<RwLock<AppState>>>) -> Result<StatusResponse, String> {
    let state = state.read();
    Ok(StatusResponse {
        status: format!("{:?}", state.pipeline.status).to_lowercase(),
        current_mode: format!("{:?}", state.pipeline.current_mode).to_lowercase(),
        is_capturing: state.audio.is_capturing,
        whisper_time_ms: state.pipeline.whisper_time_ms,
        translate_time_ms: state.pipeline.translate_time_ms,
        tts_time_ms: state.pipeline.tts_time_ms,
    })
}

#[tauri::command]
pub fn get_transcription(state: State<'_, Arc<RwLock<AppState>>>) -> Result<String, String> {
    let state = state.read();
    Ok(state.pipeline.transcription.clone())
}

#[tauri::command]
pub fn get_translation(state: State<'_, Arc<RwLock<AppState>>>) -> Result<String, String> {
    let state = state.read();
    Ok(state.pipeline.translation.clone())
}

#[tauri::command]
pub fn get_settings(state: State<'_, Arc<RwLock<AppState>>>) -> Result<SettingsResponse, String> {
    let state = state.read();
    Ok(SettingsResponse {
        source_language: state.settings.source_language.clone(),
        target_language: state.settings.target_language.clone(),
        input_device: state.settings.input_device.clone(),
        output_device: state.settings.output_device.clone(),
        operation_mode: format!("{:?}", state.settings.operation_mode).to_lowercase(),
        live_pause_threshold_ms: state.settings.live_pause_threshold_ms,
        volume: state.settings.volume,
        auto_play: state.settings.auto_play,
    })
}

#[tauri::command]
pub fn save_settings(settings: SettingsResponse) -> Result<(), String> {
    let settings = config::Settings {
        source_language: settings.source_language,
        target_language: settings.target_language,
        input_device: settings.input_device,
        output_device: settings.output_device,
        operation_mode: match settings.operation_mode.as_str() {
            "automatic" => OperationMode::Automatic,
            "manual" => OperationMode::Manual,
            "live" => OperationMode::Live,
            "transcription" => OperationMode::Transcription,
            _ => OperationMode::Automatic,
        },
        live_pause_threshold_ms: settings.live_pause_threshold_ms,
        volume: settings.volume,
        auto_play: settings.auto_play,
        models_path: None,
    };
    config::save_settings(&settings)
}

#[tauri::command]
pub fn load_models(models_path: String) -> Result<(), String> {
    info!("Loading models from: {}", models_path);
    Ok(())
}

#[tauri::command]
pub fn unload_models() -> Result<(), String> {
    info!("Unloading models");
    Ok(())
}