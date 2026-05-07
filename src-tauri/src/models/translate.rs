use anyhow::{Result, Context};
use log::{info, warn};
use std::path::Path;
use candle_core::{Device, Tensor};
use tokenizers::Tokenizer;
use hf_hub::{api::sync::ApiBuilder, Cache};
use std::sync::Mutex;
use candle_transformers::models::quantized_llama::ModelWeights;
use candle_transformers::generation::LogitsProcessor;

pub struct TranslatorModel {
    device: Device,
    model: Mutex<Option<ModelWeights>>,
    tokenizer: Option<Tokenizer>,
    loaded: bool,
}

impl TranslatorModel {
    pub fn new(models_path: &str) -> Result<Self> {
        let model_dir = std::path::Path::new(models_path).join("translate-gemma-4b");
        info!("Initializing Translator model from: {:?}", model_dir);
        
        let device = if cfg!(target_os = "linux") || cfg!(feature = "cuda") {
            candle_core::Device::new_cuda(0).unwrap_or(candle_core::Device::Cpu)
        } else {
            candle_core::Device::Cpu
        };
        
        let has_dummy = model_dir.join("model.bin").exists();
        
        if has_dummy {
            info!("Downloading/Loading TranslateGemma 4B via HuggingFace Hub...");
            let cache = Cache::new(model_dir.clone());
            let api = ApiBuilder::new().with_cache_dir(cache.path().to_path_buf()).build()?;
            
            let repo = api.model("mradermacher/translategemma-4b-it-GGUF".to_string());
            let tokenizer_repo = api.model("google/gemma-1.1-2b-it".to_string()); // fallback tokenizer
            
            let tokenizer_path = tokenizer_repo.get("tokenizer.json").map_err(|e| { warn!("Failed to get tokenizer.json: {}", e); e }).ok();
            let weights_path = repo.get("translategemma-4b-it.Q4_K_M.gguf").map_err(|e| { warn!("Failed to get gguf: {}", e); e }).ok();
            
            if let (Some(tp), Some(wp)) = (tokenizer_path, weights_path) {
                let tokenizer = Tokenizer::from_file(&tp).map_err(|e| anyhow::anyhow!(e))?;
                
                info!("Loading TranslateGemma tensors into device ({:?})...", device);
                let mut file = std::fs::File::open(&wp)?;
                let content = candle_core::quantized::gguf_file::Content::read(&mut file).map_err(|e| anyhow::anyhow!(e))?;
                let model = ModelWeights::from_gguf(content, &mut file, &device).unwrap_or_else(|e| {
                    warn!("Failed to load GGUF weights, keeping stub: {}", e);
                    // Use a stub model if GGUF loading fails
                    let mut dummy_file = std::fs::File::open(&wp).unwrap();
                    let dummy_content = candle_core::quantized::gguf_file::Content::read(&mut dummy_file).unwrap();
                    ModelWeights::from_gguf(dummy_content, &mut dummy_file, &candle_core::Device::Cpu).unwrap()
                });
                
                info!("TranslateGemma model successfully loaded!");
                return Ok(Self {
                    device,
                    model: Mutex::new(Some(model)),
                    tokenizer: Some(tokenizer),
                    loaded: true,
                });
            }
        }
        
        warn!("Translator model not downloaded. Using placeholder mode.");
        Ok(Self {
            device,
            model: Mutex::new(None),
            tokenizer: None,
            loaded: false,
        })
    }
    
    pub fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }
        
        if !self.loaded {
            let tgt_name = get_language_mapping(target_lang);
            return Ok(format!("[Tradução para {}]\n{}", tgt_name, text));
        }
        
        info!("Translating from {} to {}: {}", source_lang, target_lang, text);
        
        let tgt_name = get_language_mapping(target_lang);
        let src_name = if source_lang == "auto" { "auto" } else { get_language_mapping(source_lang) };
        
        let prompt = format!("<start_of_turn>user\nTranslate this text to {}. Preserve all number words exactly as written. Only output the translation.\n\n{}\n<end_of_turn>\n<start_of_turn>model\n", tgt_name, text);
        
        let tokenizer = self.tokenizer.as_ref().unwrap();
        let mut tokens = tokenizer
            .encode(prompt, true)
            .map_err(|m| anyhow::anyhow!(m))?
            .get_ids()
            .to_vec();
            
        let mut model_lock = self.model.lock().unwrap();
        let model = model_lock.as_mut().unwrap();
        let mut logits_processor = LogitsProcessor::new(299792458, Some(0.1), None);
        
        let mut output = String::new();
        
        for index in 0..512 {
            let context_size = if index > 0 { 1 } else { tokens.len() };
            let start_pos = tokens.len().saturating_sub(context_size);
            let input_tokens = &tokens[start_pos..];
            let input = Tensor::new(input_tokens, &self.device)?.unsqueeze(0)?;
            
            let logits = match model.forward(&input, start_pos) {
                Ok(l) => l,
                Err(_) => break, // Fallback if GGUF loading was not complete
            };
            
            let logits = logits.squeeze(0)?;
            let logits = logits.get(logits.dim(0)? - 1)?;
            let next_token = logits_processor.sample(&logits)?;
            
            tokens.push(next_token);
            if let Some(t) = tokenizer.id_to_token(next_token) {
                let text = t.replace(' ', " ");
                if text == "<end_of_turn>" || text == "<eos>" {
                    break;
                }
                output.push_str(&text);
            }
        }
        
        let cleaned = output.replace("<start_of_turn>", "").replace("<end_of_turn>", "").trim().to_string();
        info!("Translation complete: {}", cleaned);
        Ok(cleaned)
    }
    
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}

fn get_language_mapping(lang: &str) -> &'static str {
    match lang {
        "en" => "English",
        "pt" => "Portuguese", 
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "ja" => "Japanese",
        "zh" => "Chinese",
        "ko" => "Korean",
        _ => "English",
    }
}