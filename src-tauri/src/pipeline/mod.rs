use log::info;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::app_state::{OperationMode, PipelineState, PipelineStatus};
use crate::config::Settings;
use crate::models::{TtsModel, TranslateModel, WhisperModelHandler};

pub struct Pipeline {
    whisper: WhisperModelHandler,
    translate: TranslateModel,
    tts: TtsModel,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            whisper: WhisperModelHandler::new(),
            translate: TranslateModel::new(),
            tts: TtsModel::new(),
        }
    }

    pub fn process(
        &mut self,
        audio_data: &[f32],
        sample_rate: u32,
        source_lang: &str,
        target_lang: &str,
        settings: &Settings,
    ) -> Result<PipelineResult, String> {
        let start = std::time::Instant::now();

        info!("Starting pipeline processing");

        let mut state = PipelineState::default();
        state.status = PipelineStatus::Transcribing;

        // Step 1: Transcribe
        let whisper_start = std::time::Instant::now();
        let transcription = self.whisper.transcribe(audio_data, sample_rate)?;
        state.whisper_time_ms = whisper_start.elapsed().as_millis() as u64;
        state.transcription = transcription.clone();
        state.status = PipelineStatus::Translating;

        // Step 2: Translate
        let translate_start = std::time::Instant::now();
        let translation = self.translate.translate(&transcription, source_lang, target_lang)?;
        state.translate_time_ms = translate_start.elapsed().as_millis() as u64;
        state.translation = translation.clone();

        // Step 3: TTS (if not in transcription mode)
        let tts_audio = if settings.operation_mode != OperationMode::Transcription {
            state.status = PipelineStatus::Synthesizing;
            let tts_start = std::time::Instant::now();
            let audio = self.tts.synthesize(&translation)?;
            state.tts_time_ms = tts_start.elapsed().as_millis() as u64;
            Some(audio)
        } else {
            None
        };

        state.status = PipelineStatus::Idle;

        let total_time_ms = start.elapsed().as_millis() as u64;
        info!("Pipeline complete in {}ms", total_time_ms);

        Ok(PipelineResult {
            transcription,
            translation,
            tts_audio,
            whisper_time_ms: state.whisper_time_ms,
            translate_time_ms: state.translate_time_ms,
            tts_time_ms: state.tts_time_ms,
            total_time_ms,
        })
    }

    pub fn load_whisper(&mut self, model_path: &str) -> Result<(), String> {
        self.whisper.load(model_path)
    }

    pub fn load_translate(&mut self, model_path: &str) -> Result<(), String> {
        self.translate.load(model_path)
    }

    pub fn load_tts(&mut self, model_path: &str, sample_path: &str) -> Result<(), String> {
        self.tts.load(model_path, sample_path)
    }

    pub fn unload_all(&mut self) {
        self.whisper.unload();
        self.translate.unload();
        self.tts.unload();
    }

    pub fn is_whisper_loaded(&self) -> bool {
        self.whisper.is_loaded()
    }

    pub fn is_translate_loaded(&self) -> bool {
        self.translate.is_loaded()
    }

    pub fn is_tts_loaded(&self) -> bool {
        self.tts.is_loaded()
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PipelineResult {
    pub transcription: String,
    pub translation: String,
    pub tts_audio: Option<Vec<u8>>,
    pub whisper_time_ms: u64,
    pub translate_time_ms: u64,
    pub tts_time_ms: u64,
    pub total_time_ms: u64,
}