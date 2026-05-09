use crate::config::AppConfig;
use crate::hardware::HardwareInfo;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OperationMode {
    Auto,
    Manual,
    Live,
    Transcription,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppStatus {
    Idle,
    Loading,
    Ready,
    Recording,
    Processing,
    Error(String),
}

#[derive(Clone)]
pub struct AppStateData {
    pub status: AppStatus,
    pub mode: OperationMode,
    pub source_language: String,
    pub target_language: String,
    pub hardware_info: HardwareInfo,
    pub config: AppConfig,
    pub last_transcription: Option<String>,
    pub last_translation: Option<String>,
    pub last_audio: Option<Vec<f32>>,
    pub pipeline: Option<Arc<crate::pipeline::Pipeline>>,
    pub stt_time: u32,
    pub translation_time: u32,
    pub tts_time: u32,
}

impl Default for AppStateData {
    fn default() -> Self {
        Self {
            status: AppStatus::Loading,
            mode: OperationMode::Auto,
            source_language: "auto".to_string(),
            target_language: "en".to_string(),
            hardware_info: HardwareInfo::detect(),
            config: AppConfig::default(),
            last_transcription: Some("Carregando modelos...".to_string()),
            last_translation: Some("Aguarde...".to_string()),
            last_audio: None,
            pipeline: None,
            stt_time: 0,
            translation_time: 0,
            tts_time: 0,
        }
    }
}

pub type AppState = Arc<Mutex<AppStateData>>;

pub fn init_app_state(app_handle: &tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(Mutex::new(AppStateData::default()));
    app_handle.manage(state.clone());
    
    let models_path = crate::models::downloader::create_models_directory().unwrap_or_else(|_| "models".to_string());
    
    tauri::async_runtime::spawn(async move {
        let pipeline = crate::pipeline::Pipeline::new();
        let res = pipeline.load_models(&models_path).await.map_err(|e| e.to_string());
        if let Err(e_string) = res {
            log::error!("Failed to load models: {}", e_string);
            let mut data = state.lock().await;
            data.status = AppStatus::Error(format!("Model load failed: {}", e_string));
            return;
        }
        
        let mut data = state.lock().await;
        data.pipeline = Some(Arc::new(pipeline));
        data.status = AppStatus::Ready;
        data.last_transcription = Some("Sistema Pronto".to_string());
        data.last_translation = Some("Modelos Carregados".to_string());
    });
    
    Ok(())
}

pub fn get_state(app_handle: &tauri::AppHandle) -> &AppState {
    app_handle.state::<AppState>().inner()
}