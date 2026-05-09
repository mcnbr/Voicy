use anyhow::Result;
use log::{info, warn};
use candle_core::{Device, Tensor};
use tokenizers::Tokenizer;
use hf_hub::api::sync::ApiBuilder;
use std::sync::Mutex;
use candle_transformers::models::quantized_gemma3::ModelWeights;
use candle_transformers::generation::LogitsProcessor;

pub struct TranslatorModel {
    model: Mutex<Option<ModelWeights>>,
    tokenizer: Option<Tokenizer>,
    device: Device,
    loaded: bool,
}

impl TranslatorModel {
    pub fn new(models_path: &str) -> Result<Self> {
        let mut model_instance = Self {
            model: Mutex::new(None),
            tokenizer: None,
            device: Device::Cpu,
            loaded: false,
        };
        
        let model_dir = std::path::Path::new(models_path).join("translate-gemma-4b");
            
        std::fs::create_dir_all(&model_dir)?;
        info!("Initializing Translator model from: {:?}", model_dir);
        
        model_instance.device = if candle_core::utils::cuda_is_available() {
            info!("Translator: Using CUDA GPU");
            Device::new_cuda(0).unwrap_or(Device::Cpu)
        } else {
            info!("Translator: Using CPU");
            Device::Cpu
        };
        
        info!("Downloading/Loading TranslateGemma-4b via HuggingFace Hub...");
        let api = ApiBuilder::new()
            .with_cache_dir(model_dir.clone())
            .build()
            .unwrap();
            
        let repo_weights = api.model("mradermacher/translategemma-4b-it-GGUF".to_string());
        let weights_path = match repo_weights.get("translategemma-4b-it.Q4_K_M.gguf") {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to download translator weights: {}", e);
                return Ok(model_instance);
            }
        };
        
        let repo_tokenizer = api.model("mlx-community/translategemma-4b-it-4bit".to_string());
        let tokenizer_path = match repo_tokenizer.get("tokenizer.json") {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to download translator tokenizer: {}", e);
                return Ok(model_instance);
            }
        };
        
        model_instance.tokenizer = Some(Tokenizer::from_file(&tokenizer_path).map_err(|e| anyhow::anyhow!(e))?);
        
        info!("Loading TranslateGemma tensors into device ({:?})...", model_instance.device);
        let mut file = std::fs::File::open(&weights_path)?;
        let gguf_content = candle_core::quantized::gguf_file::Content::read(&mut file)?;
        let weights = match ModelWeights::from_gguf(gguf_content, &mut file, &model_instance.device) {
            Ok(w) => w,
            Err(e) => {
                log::error!("TranslateGemma load error (architecture unsupported?): {}", e);
                return Ok(model_instance);
            }
        };
        
        *model_instance.model.lock().unwrap() = Some(weights);
        model_instance.loaded = true;
        info!("TranslateGemma model successfully loaded!");
        
        Ok(model_instance)
    }
    
    pub fn translate(&self, text: &str, _src_lang: &str, tgt_lang: &str) -> Result<String> {
        if !self.loaded {
            return Ok(format!("[Translator not loaded] {}", text));
        }
        
        let tgt_name = match tgt_lang {
            "en" => "English",
            "pt" => "Portuguese",
            "es" => "Spanish",
            _ => "English"
        };
        
        // TranslateGemma prompt format
        let prompt = format!("<start_of_turn>user\nTranslate the following text to {}:\n\n{}<end_of_turn>\n<start_of_turn>model\n", tgt_name, text);
        
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
                Err(e) => {
                    log::error!("Translate forward error: {:?}", e);
                    break;
                }
            };
            
            let logits = logits.squeeze(0)?;
            
            let next_token = logits_processor.sample(&logits)?;
            
            tokens.push(next_token);
            if let Some(t) = tokenizer.id_to_token(next_token) {
                let text = t.replace('\u{2581}', " ").replace('\u{0120}', " ").replace(' ', " ");
                if text.contains("<end_of_turn>") || text.contains("<eos>") {
                    break;
                }
                output.push_str(&text);
            }
        }
        
        let cleaned = output.replace("<start_of_turn>", "").replace("<end_of_turn>", "").replace("<eos>", "").trim().to_string();
        info!("Translation complete: {}", cleaned);
        Ok(cleaned)
    }
    
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
}