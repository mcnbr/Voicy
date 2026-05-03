pub mod app_state;
pub mod audio;
pub mod commands;
pub mod config;
pub mod hardware;
pub mod models;
pub mod pipeline;

use std::sync::Arc;
use log::{info, error};
use tauri::Manager;
use parking_lot::RwLock;

use app_state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("Voicy v{} starting...", env!("CARGO_PKG_VERSION"));

    let app_state = Arc::new(RwLock::new(AppState::default()));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            commands::get_hardware_info,
            commands::get_audio_devices,
            commands::set_input_device,
            commands::set_output_device,
            commands::set_source_language,
            commands::set_target_language,
            commands::start_capture,
            commands::stop_capture,
            commands::get_status,
            commands::get_transcription,
            commands::get_translation,
            commands::get_settings,
            commands::save_settings,
            commands::load_models,
            commands::unload_models,
        ])
        .setup(move |app| {
            info!("Tauri app setup complete");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}