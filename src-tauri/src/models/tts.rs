use anyhow::Result;
use log::{info, warn};
use std::sync::Mutex;

use omnivoice_infer::pipeline::Phase3Pipeline;
use omnivoice_infer::contracts::GenerationRequest;
use omnivoice_infer::runtime::{RuntimeOptions, DeviceSpec};

pub struct TtsModel {
    pipeline: Mutex<Option<Phase3Pipeline>>,
    loaded: bool,
    sample_rate: u32,
}

impl TtsModel {
    pub fn new(models_path: &str) -> Result<Self> {
        let model_dir = std::path::Path::new(models_path).join("omnivoice");
        info!("Initializing TTS model from: {:?}", model_dir);
        
        let has_model = model_dir.join("omnivoice.artifacts.json").exists();
        
        if !has_model {
            warn!("TTS model not found at: {:?}. Using placeholder audio.", model_dir);
            return Ok(Self {
                pipeline: Mutex::new(None),
                loaded: false,
                sample_rate: 24000,
            });
        }
        
        {
            // Probe CUDA at runtime — DeviceSpec::Cuda will be used if a GPU is found
            let device = match candle_core::Device::new_cuda(0) {
                Ok(_) => { info!("TTS: Using CUDA GPU"); DeviceSpec::Cuda(0) }
                Err(_) => { info!("TTS: Using CPU"); DeviceSpec::Cpu }
            };
            
            let options = RuntimeOptions::new(model_dir.clone())
                .with_device(device);
                
            match Phase3Pipeline::from_options(options) {
                Ok(pipeline) => {
                    info!("TTS model found and loaded");
                    return Ok(Self {
                        pipeline: Mutex::new(Some(pipeline)),
                        loaded: true,
                        sample_rate: 24000,
                    });
                }
                Err(e) => {
                    warn!("Failed to load OmniVoice pipeline, keeping stub: {}", e);
                }
            }
        }

        Ok(Self {
            pipeline: Mutex::new(None),
            loaded: false,
            sample_rate: 24000,
        })
    }
    
    pub fn synthesize(&self, text: &str) -> Result<Vec<f32>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }
        
        if !self.loaded {
            info!("TTS model not loaded, returning silence");
            return Ok(self.generate_placeholder_audio(24000));
        }
        
        info!("Synthesizing text: {}", text);
        
        {
            let req = GenerationRequest::new_text_only(text);
            let mut pipeline_lock = self.pipeline.lock().unwrap();
            if let Some(pipeline) = pipeline_lock.as_mut() {
                let audio_results = pipeline.generate(&req).map_err(|e| anyhow::anyhow!(e))?;
                
                if let Some(audio) = audio_results.into_iter().next() {
                    info!("Synthesis complete: {} samples generated", audio.samples.len());
                    return Ok(audio.samples);
                }
            }
        }
        
        let word_count = text.split_whitespace().count();
        let duration_secs = (word_count as f32 * 0.15).max(1.0);
        let num_samples = (self.sample_rate as f32 * duration_secs) as usize;
        let samples = self.generate_placeholder_audio(num_samples);
        info!("Synthesis complete: {} placeholder samples generated", samples.len());
        Ok(samples)
    }
    
    fn generate_placeholder_audio(&self, num_samples: usize) -> Vec<f32> {
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / self.sample_rate as f32;
            let freq = 440.0 + (t * 10.0);
            let sample = (2.0 * std::f32::consts::PI * freq * t).sin() * 0.1;
            samples.push(sample);
        }
        samples
    }
    
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }
    
    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }
}