use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub models_path: String,
    pub audio_input_device: Option<String>,
    pub audio_output_device: Option<String>,
    pub whisper_model: String,
    pub translate_model: String,
    pub tts_model: String,
    pub vad_threshold: f32,
    pub vad_min_speech_duration_ms: u32,
    pub batch_size: u32,
    pub live_translation_delay_ms: u32,
    pub auto_play_translation: bool,
    pub clipboard_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            models_path: "models".to_string(),
            audio_input_device: None,
            audio_output_device: None,
            whisper_model: "whisper-large-v3-turbo".to_string(),
            translate_model: "translate-gemma-4b".to_string(),
            tts_model: "omnivoice".to_string(),
            vad_threshold: 0.5,
            vad_min_speech_duration_ms: 250,
            batch_size: 16,
            live_translation_delay_ms: 500,
            auto_play_translation: true,
            clipboard_mode: false,
        }
    }
}

impl AppConfig {
    pub fn load_from_file(path: &str) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}