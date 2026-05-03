use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    pub hardware: HardwareInfo,
    pub audio: AudioState,
    pub pipeline: PipelineState,
    pub models: ModelsState,
    pub settings: Settings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub has_cuda: bool,
    pub cuda_device: Option<String>,
    pub vram_used_mb: u64,
    pub vram_available_mb: u64,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioState {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub is_capturing: bool,
    pub is_playing: bool,
    pub volume: f32,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineState {
    pub status: PipelineStatus,
    pub current_mode: OperationMode,
    pub transcription: String,
    pub translation: String,
    pub whisper_time_ms: u64,
    pub translate_time_ms: u64,
    pub tts_time_ms: u64,
    pub audio_buffer: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    #[default]
    Idle,
    Capturing,
    Transcribing,
    Translating,
    Synthesizing,
    Playing,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OperationMode {
    #[default]
    Automatic,
    Manual,
    Live,
    Transcription,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelsState {
    pub whisper_loaded: bool,
    pub translate_loaded: bool,
    pub tts_loaded: bool,
    pub whisper_model: Option<String>,
    pub translate_model: Option<String>,
    pub tts_model: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub source_language: String,
    pub target_language: String,
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub operation_mode: OperationMode,
    pub live_pause_threshold_ms: u64,
    pub volume: f32,
    pub auto_play: bool,
    pub models_path: Option<String>,
}