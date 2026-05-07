use anyhow::{Result, Context};
use log::{info, warn};
use std::path::Path;
use candle_core::{Device, Tensor, IndexOp};
use candle_nn::ops::softmax;
use candle_transformers::models::whisper::{self as m, audio, Config};
use tokenizers::Tokenizer;
use hf_hub::{api::sync::ApiBuilder, Cache};
use std::sync::Mutex;

pub fn token_id(tokenizer: &Tokenizer, token: &str) -> candle_core::Result<u32> {
    match tokenizer.token_to_id(token) {
        None => candle_core::bail!("no token-id for {}", token),
        Some(id) => Ok(id),
    }
}

pub struct WhisperModel {
    device: Device,
    model: Mutex<Option<m::quantized_model::Whisper>>,
    tokenizer: Option<Tokenizer>,
    config: Option<Config>,
    mel_filters: Vec<f32>,
    loaded: bool,
}

impl WhisperModel {
    pub fn new(models_path: &str) -> Result<Self> {
        let model_dir = std::path::Path::new(models_path).join("whisper-large-v3-turbo");
        info!("Initializing Whisper model from: {:?}", model_dir);
        
        let device = if cfg!(target_os = "linux") || cfg!(feature = "cuda") {
            candle_core::Device::new_cuda(0).unwrap_or(candle_core::Device::Cpu)
        } else {
            candle_core::Device::Cpu
        };
        
        let mel_bytes = include_bytes!("melfilters128.bytes").as_slice();
        let mut mel_filters = vec![0f32; mel_bytes.len() / 4];
        <byteorder::LittleEndian as byteorder::ByteOrder>::read_f32_into(mel_bytes, &mut mel_filters);

        let has_dummy = model_dir.join("model.bin").exists();
        
        if has_dummy {
            info!("Downloading/Loading Whisper Large V3 Turbo via HuggingFace Hub...");
            let cache = Cache::new(model_dir.clone());
            let api = ApiBuilder::new().with_cache_dir(cache.path().to_path_buf()).build()?;
            let repo_gguf = api.model("QuantFactory/Whisper-Large-V3-Turbo-GGUF".to_string());
            let repo_orig = api.model("openai/whisper-large-v3-turbo".to_string());
            
            info!("Fetching config.json...");
            let config_path = repo_orig.get("config.json").map_err(|e| { warn!("Failed to get config.json: {}", e); e }).ok();
            info!("Fetching tokenizer.json...");
            let tokenizer_path = repo_orig.get("tokenizer.json").map_err(|e| { warn!("Failed to get tokenizer.json: {}", e); e }).ok();
            info!("Fetching weights (1.5GB, this might take a while)...");
            let weights_path = repo_gguf.get("whisper-large-v3-turbo.Q5_K_M.gguf").map_err(|e| { warn!("Failed to get gguf: {}", e); e }).ok();
            
            if let (Some(cp), Some(tp), Some(wp)) = (config_path, tokenizer_path, weights_path) {
                info!("All files downloaded. Parsing configuration...");
                let config: Config = serde_json::from_str(&std::fs::read_to_string(&cp)?)?;
                let tokenizer = Tokenizer::from_file(&tp).map_err(|e| anyhow::anyhow!(e))?;
                
                info!("Loading tensors into device ({:?})...", device);
                let vb = candle_transformers::quantized_var_builder::VarBuilder::from_gguf(&wp, &device)?;
                let model = m::quantized_model::Whisper::load(&vb, config.clone())?;
                
                info!("Whisper model successfully loaded!");
                return Ok(Self {
                    device,
                    model: Mutex::new(Some(model)),
                    tokenizer: Some(tokenizer),
                    config: Some(config),
                    mel_filters,
                    loaded: true,
                });
            }
        }
        
        warn!("Whisper model not downloaded. Using placeholder mode.");
        Ok(Self {
            device,
            model: Mutex::new(None),
            tokenizer: None,
            config: None,
            mel_filters,
            loaded: false,
        })
    }
    
    pub fn transcribe(&self, audio_samples: &[f32]) -> Result<String> {
        if !self.loaded {
            return Ok("[Transcrição] Modelo Whisper não carregado. Baixe o modelo para ativar.".to_string());
        }
        
        if audio_samples.is_empty() {
            return Ok(String::new());
        }
        
        info!("Transcribing {} audio samples", audio_samples.len());
        let config = self.config.as_ref().unwrap();
        let tokenizer = self.tokenizer.as_ref().unwrap();
        
        let mel = audio::pcm_to_mel(config, audio_samples, &self.mel_filters);
        let mel_len = mel.len();
        let mel = Tensor::from_vec(
            mel,
            (1, config.num_mel_bins, mel_len / config.num_mel_bins),
            &self.device,
        )?;
        
        let mut model_lock = self.model.lock().unwrap();
        let model = model_lock.as_mut().unwrap();
        
        let audio_features = model.encoder.forward(&mel, true)?;
        
        let mut tokens = vec![
            token_id(tokenizer, m::SOT_TOKEN)?,
            token_id(tokenizer, "<|pt|>").unwrap_or(token_id(tokenizer, m::SOT_TOKEN)?),
            token_id(tokenizer, m::TRANSCRIBE_TOKEN)?,
            token_id(tokenizer, m::NO_TIMESTAMPS_TOKEN)?,
        ];
        
        let sample_len = config.max_target_positions / 2;
        let eot_token = token_id(tokenizer, m::EOT_TOKEN)?;
        
        for i in 0..sample_len {
            let tokens_t = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;
            let ys = model.decoder.forward(&tokens_t, &audio_features, i == 0)?;
            let (_, seq_len, _) = ys.dims3()?;
            let logits = model.decoder.final_linear(&ys.i((..1, seq_len - 1..))?)?.i(0)?.i(0)?;
            
            let next_token = logits.argmax(0)?.to_scalar::<u32>()?;
            tokens.push(next_token);
            
            if next_token == eot_token || tokens.len() > config.max_target_positions {
                break;
            }
        }
        
        let text = tokenizer.decode(&tokens, true).map_err(|e| anyhow::anyhow!(e))?;
        info!("Transcription complete");
        
        Ok(text)
    }
    
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
    
    pub fn get_device(&self) -> &Device {
        &self.device
    }
}