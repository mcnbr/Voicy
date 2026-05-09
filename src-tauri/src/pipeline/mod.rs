use crate::models::ModelManager;
use log::info;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct Pipeline {
    pub model_manager: Arc<tokio::sync::Mutex<ModelManager>>,
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

    pub async fn process_audio(&self, audio: Vec<f32>, source_lang: &str, target_lang: &str) -> Result<PipelineResult, Box<dyn std::error::Error>> {
        let manager = self.model_manager.lock().await;

        info!("Pipeline: Starting audio processing");

        let start = std::time::Instant::now();
        let transcription = if let Some(whisper) = manager.get_whisper() {
            whisper.transcribe(&audio, source_lang)?
        } else {
            "No whisper model loaded".to_string()
        };
        let stt_time = start.elapsed().as_millis() as u32;

        let start = std::time::Instant::now();
        let translation = if transcription.trim().is_empty() {
            String::new()
        } else if let Some(translator) = manager.get_translator() {
            translator.translate(&transcription, source_lang, target_lang)?
        } else {
            "No translator model loaded".to_string()
        };
        let translation_time = start.elapsed().as_millis() as u32;

        let ref_audio = crate::audio::get_last_recording_for_tts();

        let start = std::time::Instant::now();
        let audio_output = if let Some(tts) = manager.get_tts() {
            tts.synthesize(&translation, ref_audio)?
        } else {
            Vec::new()
        };
        let tts_time = start.elapsed().as_millis() as u32;

        Ok(PipelineResult {
            original_text: transcription,
            translated_text: translation,
            audio_output,
            stt_time,
            translation_time,
            tts_time,
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
    pub stt_time: u32,
    pub translation_time: u32,
    pub tts_time: u32,
}