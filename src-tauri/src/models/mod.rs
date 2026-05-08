pub mod whisper;
pub mod translate;
pub mod tts;
pub mod downloader;

use log::info;

pub struct ModelManager {
    whisper: Option<whisper::WhisperModel>,
    translator: Option<translate::TranslatorModel>,
    tts: Option<tts::TtsModel>,
    models_loaded: bool,
}

impl ModelManager {
    pub fn new() -> Self {
        Self {
            whisper: None,
            translator: None,
            tts: None,
            models_loaded: false,
        }
    }

    pub async fn load_models(&mut self, models_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading AI models from: {}", models_path);

        info!("Loading Whisper model...");
        self.whisper = Some(whisper::WhisperModel::new(models_path)?);
        info!("Whisper model loaded");

        info!("Loading Translator model...");
        self.translator = Some(translate::TranslatorModel::new(models_path)?);
        info!("Translator model loaded");

        info!("Loading TTS model...");
        self.tts = Some(tts::TtsModel::new(models_path)?);
        info!("TTS model loaded");

        self.models_loaded = true;
        info!("All models loaded successfully");

        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.models_loaded
    }

    pub fn get_whisper(&self) -> Option<&whisper::WhisperModel> {
        self.whisper.as_ref()
    }

    pub fn get_translator(&self) -> Option<&translate::TranslatorModel> {
        self.translator.as_ref()
    }

    pub fn get_tts(&self) -> Option<&tts::TtsModel> {
        self.tts.as_ref()
    }

    /// Returns the device name actually used by the primary model (Whisper)
    pub fn get_active_device(&self) -> String {
        self.whisper
            .as_ref()
            .map(|w| w.device_name().to_string())
            .unwrap_or_else(|| "CPU".to_string())
    }

    pub fn unload(&mut self) {
        self.whisper = None;
        self.translator = None;
        self.tts = None;
        self.models_loaded = false;
        info!("Models unloaded");
    }
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}