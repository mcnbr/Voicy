use log::info;
use std::path::Path;
use std::fs;

pub struct ModelDownloader {
    models_path: String,
}

impl ModelDownloader {
    pub fn new(models_path: &str) -> Self {
        Self {
            models_path: models_path.to_string(),
        }
    }

    pub fn check_models_exist(&self) -> bool {
        let whisper_path = Path::new(&self.models_path).join("whisper-large-v3-turbo");
        let translate_path = Path::new(&self.models_path).join("translate-gemma-4b");
        let tts_path = Path::new(&self.models_path).join("omnivoice");

        whisper_path.exists() || translate_path.exists() || tts_path.exists()
    }

    pub fn get_model_info(&self) -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                name: "Whisper Large V3 Turbo".to_string(),
                folder: "whisper-large-v3-turbo".to_string(),
                expected_size_mb: 1500,
                description: "Speech-to-Text transcription model".to_string(),
                status: self.get_status("whisper-large-v3-turbo"),
            },
            ModelInfo {
                name: "TranslateGemma 4B".to_string(),
                folder: "translate-gemma-4b".to_string(),
                expected_size_mb: 8000,
                description: "Machine translation model".to_string(),
                status: self.get_status("translate-gemma-4b"),
            },
            ModelInfo {
                name: "OmniVoice".to_string(),
                folder: "omnivoice".to_string(),
                expected_size_mb: 500,
                description: "Text-to-Speech synthesis".to_string(),
                status: self.get_status("omnivoice"),
            },
        ]
    }

    fn get_status(&self, folder: &str) -> ModelStatus {
        let path = Path::new(&self.models_path).join(folder);
        if path.exists() {
            if let Ok(entries) = fs::read_dir(&path) {
                let count = entries.count();
                if count > 0 {
                    return ModelStatus::Downloaded;
                }
            }
        }
        ModelStatus::NotDownloaded
    }

    pub fn get_models_path(&self) -> &str {
        &self.models_path
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub folder: String,
    pub expected_size_mb: u32,
    pub description: String,
    pub status: ModelStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ModelStatus {
    NotDownloaded,
    Downloading,
    Downloaded,
    Error,
}

pub fn create_models_directory() -> Result<String, Box<dyn std::error::Error>> {
    let exe_dir = std::env::current_exe()?
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();

    let models_dir = exe_dir.join("models");
    
    if !models_dir.exists() {
        fs::create_dir_all(&models_dir)?;
        info!("Created models directory: {:?}", models_dir);
    }

    Ok(models_dir.to_string_lossy().to_string())
}

pub fn get_download_links() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "Whisper Large V3 Turbo",
            "https://huggingface.co/QuantFactory/Whisper-Large-V3-Turbo-GGUF/resolve/main/whisper-large-v3-turbo.Q5_K_M.gguf",
            "whisper-large-v3-turbo",
        ),
        (
            "TranslateGemma 4B", 
            "https://huggingface.co/google/translate-gemma-2b-gguf/resolve/main/translate-gemma-2b-q4_0.gguf",
            "translate-gemma-4b",
        ),
        (
            "OmniVoice",
            "https://huggingface.co/FerrisMind/omnivoice-gguf/resolve/main/omnivoice-q4_0.gguf",
            "omnivoice",
        ),
    ]
}