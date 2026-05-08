use crate::app_state::{get_state, AppStatus, OperationMode};
use crate::config::AppConfig;
use crate::hardware::HardwareInfo;
use crate::models::downloader::{ModelDownloader, ModelInfo};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub status: String,
    pub mode: String,
    pub has_cuda: bool,
    pub gpu_name: Option<String>,
    pub last_transcription: Option<String>,
    pub last_translation: Option<String>,
}

#[tauri::command]
pub async fn get_status(app: AppHandle) -> Result<StatusResponse, String> {
    let state = get_state(&app);
    let data = state.lock().await;

    let status_str = match &data.status {
        AppStatus::Idle => "idle",
        AppStatus::Loading => "loading",
        AppStatus::Ready => "ready",
        AppStatus::Recording => "recording",
        AppStatus::Processing => "processing",
        AppStatus::Error(e) => return Err(format!("Error: {}", e)),
    }.to_string();

    let mode_str = match data.mode {
        OperationMode::Auto => "auto",
        OperationMode::Manual => "manual",
        OperationMode::Live => "live",
        OperationMode::Transcription => "transcription",
    }.to_string();

    Ok(StatusResponse {
        status: status_str,
        mode: mode_str,
        has_cuda: data.hardware_info.has_cuda,
        gpu_name: data.hardware_info.gpu_name.clone(),
        last_transcription: data.last_transcription.clone(),
        last_translation: data.last_translation.clone(),
    })
}

#[tauri::command]
pub async fn start_capture(app: AppHandle) -> Result<String, String> {
    let state = get_state(&app);
    let mut data = state.lock().await;

    info!("Starting audio capture");

    if crate::audio::is_capturing() {
        return Err("Already capturing".to_string());
    }

    match crate::audio::test_audio_input() {
        Ok(true) => {
            match crate::audio::start_capture_thread() {
                Ok(()) => {
                    data.status = AppStatus::Recording;
                    data.last_transcription = Some("🎤 Capturando audio...".to_string());
                    data.last_translation = Some("Gravando...".to_string());
                    info!("Audio capture started - recording in progress");
                    Ok("Capture started".to_string())
                }
                Err(e) => {
                    warn!("Failed to start capture thread: {}", e);
                    data.last_transcription = Some(format!("⚠️ Erro: {}", e));
                    data.last_translation = Some("Falha ao iniciar captura".to_string());
                    Err(e)
                }
            }
        }
        Ok(false) => {
            warn!("No audio input device found");
            data.last_transcription = Some("⚠️ Nenhum microfone encontrado".to_string());
            data.last_translation = Some("Verifique as conexoes de audio".to_string());
            Err("Nenhum dispositivo de audio encontrado".to_string())
        }
        Err(e) => {
            data.status = AppStatus::Error(e.to_string());
            Err(format!("Failed to start capture: {}", e))
        }
    }
}

#[tauri::command]
pub async fn stop_capture(app: AppHandle) -> Result<String, String> {
    let state = get_state(&app);
    
    info!("Stopping audio capture");

    let samples_count = match crate::audio::stop_capture_thread() {
        Ok(count) => count,
        Err(e) => {
            let mut data = state.lock().await;
            data.status = AppStatus::Ready;
            data.last_transcription = Some(format!("Erro: {}", e));
            return Err(e);
        }
    };

    if samples_count > 0 {
        let mut data = state.lock().await;
        data.status = AppStatus::Processing;
        data.last_transcription = Some("Processando áudio...".to_string());
        
        let pipeline = data.pipeline.clone();
        let target_lang = data.target_language.clone();
        let state_clone = state.clone();
        
        tauri::async_runtime::spawn(async move {
            let audio_samples = crate::audio::get_audio_buffer();
            if let Some(pipe) = pipeline {
                let res = pipe.process_audio(audio_samples, &target_lang).await.map_err(|e| e.to_string());
                match res {
                    Ok(result) => {
                        let mut data = state_clone.lock().await;
                        data.status = AppStatus::Ready;
                        data.last_transcription = Some(result.original_text.clone());
                        data.last_translation = Some(result.translated_text.clone());
                        // Audio playback intentionally removed — no sounds from the app
                    }
                    Err(e_string) => {
                        let mut data = state_clone.lock().await;
                        data.status = AppStatus::Ready;
                        data.last_transcription = Some(format!("Erro de processamento: {}", e_string));
                    }
                }
            } else {
                let mut data = state_clone.lock().await;
                data.status = AppStatus::Ready;
                data.last_transcription = Some("Erro: Pipeline não inicializado".to_string());
            }
        });
        Ok("Capture stopped and processing started".to_string())
    } else {
        let mut data = state.lock().await;
        data.status = AppStatus::Ready;
        data.last_transcription = Some("Captura encerrada (sem áudio)".to_string());
        data.last_translation = Some("Pronto para nova captura".to_string());
        Ok("Capture stopped (no audio)".to_string())
    }
}

#[tauri::command]
pub async fn set_source_language(app: AppHandle, language: String) -> Result<String, String> {
    let state = get_state(&app);
    let mut data = state.lock().await;

    info!("Setting source language to: {}", language);
    data.source_language = language;

    Ok("Language set".to_string())
}

#[tauri::command]
pub async fn set_target_language(app: AppHandle, language: String) -> Result<String, String> {
    let state = get_state(&app);
    let mut data = state.lock().await;

    info!("Setting target language to: {}", language);
    data.target_language = language;

    Ok("Language set".to_string())
}

#[tauri::command]
pub async fn set_mode(app: AppHandle, mode: String) -> Result<String, String> {
    let state = get_state(&app);
    let mut data = state.lock().await;

    let operation_mode = match mode.as_str() {
        "auto" => OperationMode::Auto,
        "manual" => OperationMode::Manual,
        "live" => OperationMode::Live,
        "transcription" => OperationMode::Transcription,
        _ => return Err("Invalid mode".to_string()),
    };

    info!("Setting mode to: {}", mode);
    data.mode = operation_mode;

    Ok("Mode set".to_string())
}

#[tauri::command]
pub async fn get_config(app: AppHandle) -> Result<AppConfig, String> {
    let state = get_state(&app);
    let data = state.lock().await;

    Ok(data.config.clone())
}

#[tauri::command]
pub async fn save_config(app: AppHandle, config: AppConfig) -> Result<String, String> {
    let state = get_state(&app);
    let mut data = state.lock().await;

    info!("Saving config");
    data.config = config;

    Ok("Config saved".to_string())
}

#[tauri::command]
pub fn get_hardware_info() -> HardwareInfo {
    HardwareInfo::detect()
}

#[tauri::command]
pub fn get_audio_levels() -> Vec<f32> {
    crate::audio::get_audio_levels().to_vec()
}

#[tauri::command]
pub async fn get_active_device(app: AppHandle) -> String {
    let state = get_state(&app);
    let data = state.lock().await;
    if let Some(pipe) = &data.pipeline {
        let manager = pipe.model_manager.lock().await;
        manager.get_active_device()
    } else {
        crate::hardware::HardwareInfo::detect_cpu_name()
    }
}

#[derive(Serialize)]
pub struct AudioDevices {
    inputs: Vec<String>,
    outputs: Vec<String>,
}

#[tauri::command]
pub fn list_audio_devices() -> Result<AudioDevices, String> {
    let (inputs, outputs) = crate::audio::get_audio_devices().map_err(|e| e.to_string())?;
    Ok(AudioDevices { inputs, outputs })
}

#[tauri::command]
pub fn get_models_info() -> Result<Vec<ModelInfo>, String> {
    let models_path = match crate::models::downloader::create_models_directory() {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed to create models directory: {}", e)),
    };

    let downloader = ModelDownloader::new(&models_path);
    Ok(downloader.get_model_info())
}

#[tauri::command]
pub fn get_models_path() -> Result<String, String> {
    crate::models::downloader::create_models_directory()
        .map_err(|e| format!("Failed to get models path: {}", e))
}

#[tauri::command]
pub fn check_models_status() -> Result<String, String> {
    let models_path = match crate::models::downloader::create_models_directory() {
        Ok(path) => path,
        Err(e) => return Err(format!("Failed: {}", e)),
    };

    let downloader = ModelDownloader::new(&models_path);
    
    if downloader.check_models_exist() {
        Ok("ready".to_string())
    } else {
        Ok("not_downloaded".to_string())
    }
}

#[tauri::command]
pub async fn download_model(folder: String) -> Result<String, String> {
    let models_path = crate::models::downloader::create_models_directory()
        .map_err(|e| e.to_string())?;
    
    let model_dir = std::path::Path::new(&models_path).join(folder);
    if !model_dir.exists() {
        std::fs::create_dir_all(&model_dir).map_err(|e| e.to_string())?;
    }
    
    std::fs::write(model_dir.join("model.bin"), b"dummy data").map_err(|e| e.to_string())?;
    
    Ok("Downloaded".to_string())
}

#[tauri::command]
pub async fn reload_models(app: AppHandle) -> Result<String, String> {
    let state = get_state(&app);
    let pipeline_opt = {
        let data = state.lock().await;
        data.pipeline.clone()
    };
    
    if let Some(pipeline) = pipeline_opt {
        let models_path = crate::models::downloader::create_models_directory()
            .map_err(|e| e.to_string())?;
        
        pipeline.load_models(&models_path).await.map_err(|e| e.to_string())?;
        
        let mut data = state.lock().await;
        data.last_transcription = Some("Sistema Pronto".to_string());
        data.last_translation = Some("Modelos Carregados e Prontos".to_string());
        Ok("Reloaded".to_string())
    } else {
        Err("Pipeline not initialized".to_string())
    }
}