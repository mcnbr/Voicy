pub mod app_state;
pub mod commands;
pub mod audio;
pub mod models;
pub mod pipeline;
pub mod config;
pub mod hardware;

use log::info;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    info!("Starting Voicy application...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            info!("Voicy setup initiated");

            let app_handle = app.handle().clone();
            
            if let Err(e) = app_state::init_app_state(&app_handle) {
                log::error!("Failed to initialize app state: {}", e);
            }

            audio::init_audio_buffer();
            info!("Audio buffer initialized");

            info!("Application setup complete");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::start_capture,
            commands::stop_capture,
            commands::set_source_language,
            commands::set_target_language,
            commands::set_mode,
            commands::get_config,
            commands::save_config,
            commands::get_hardware_info,
            commands::list_audio_devices,
            commands::get_models_info,
            commands::get_models_path,
            commands::check_models_status,
            commands::download_model,
            commands::reload_models,
            commands::get_audio_levels,
            commands::get_active_device,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}