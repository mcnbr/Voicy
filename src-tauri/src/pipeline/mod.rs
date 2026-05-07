use crate::models::ModelManager;
use log::info;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct Pipeline {
    model_manager: Arc<tokio::sync::Mutex<ModelManager>>,
    audio_receiver: Option<mpsc::Receiver<Vec<f32>>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            model_manager: Arc::new(tokio::sync::Mutex::new(ModelManager::new())),
            audio_receiver: None,
        }
    }

    pub async fn load_models(&self, models_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut manager = self.model_manager.lock().await;
        manager.load_models(models_path).await
    }

    pub fn set_audio_input(&mut self, receiver: mpsc::Receiver<Vec<f32>>) {
        self.audio_receiver = Some(receiver);
    }

    pub async fn process_audio(&self, audio: Vec<f32>, target_lang: &str) -> Result<PipelineResult, Box<dyn std::error::Error>> {
        let manager = self.model_manager.lock().await;

        info!("Pipeline: Starting audio processing");

        let transcription = if let Some(whisper) = manager.get_whisper() {
            whisper.transcribe(&audio)?
        } else {
            "No whisper model loaded".to_string()
        };

        let translation = if let Some(translator) = manager.get_translator() {
            translator.translate(&transcription, "auto", target_lang)?
        } else {
            "No translator model loaded".to_string()
        };

        let audio_output = if let Some(tts) = manager.get_tts() {
            tts.synthesize(&translation)?
        } else {
            Vec::new()
        };

        Ok(PipelineResult {
            original_text: transcription,
            translated_text: translation,
            audio_output,
        })
    }

    pub fn is_models_loaded(&self) -> bool {
        self.model_manager.try_lock().map(|m| m.is_loaded()).unwrap_or(false)
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub original_text: String,
    pub translated_text: String,
    pub audio_output: Vec<f32>,
}