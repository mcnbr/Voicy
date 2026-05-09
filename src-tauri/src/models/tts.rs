use anyhow::Result;
use log::{info, warn};
use std::sync::Mutex;
use std::fs;
use std::io::Write;
use regex::Regex;

#[cfg(feature = "cuda")]
use omnivoice_infer::pipeline::Phase3Pipeline;
#[cfg(feature = "cuda")]
use omnivoice_infer::contracts::{GenerationRequest, ReferenceAudioInput};
#[cfg(feature = "cuda")]
use omnivoice_infer::runtime::{RuntimeOptions, DeviceSpec};
#[cfg(feature = "cuda")]
use omnivoice_infer::model_source::resolve_tts_model_root_from_path;

pub struct TtsModel {
    #[cfg(feature = "cuda")]
    pipeline: Mutex<Option<Phase3Pipeline>>,
    #[cfg(not(feature = "cuda"))]
    _unused: (),
    loaded: bool,
    sample_rate: u32,
}

impl TtsModel {
    #[cfg(feature = "cuda")]
    pub fn new(models_path: &str) -> Result<Self> {
        let local_dir = std::path::Path::new(models_path).join("omnivoice");
        info!("Checking for local OmniVoice model at: {:?}", local_dir);

        let model_root = if local_dir.join("omnivoice.artifacts.json").exists() {
            info!("Local OmniVoice model found, using it.");
            local_dir
        } else {
            info!("Local OmniVoice model not found. Auto-downloading from HuggingFace (k2-fsa/OmniVoice)...");
            match resolve_tts_model_root_from_path(None) {
                Ok(path) => {
                    info!("OmniVoice model downloaded to: {:?}", path);
                    path
                }
                Err(e) => {
                    warn!("Failed to auto-download OmniVoice model: {}. TTS will use placeholder audio.", e);
                    return Ok(Self {
                        pipeline: Mutex::new(None),
                        loaded: false,
                        sample_rate: 24000,
                    });
                }
            }
        };

        let device = match candle_core::Device::new_cuda(0) {
            Ok(_) => { info!("TTS: Using CUDA GPU"); DeviceSpec::Cuda(0) }
            Err(_) => { info!("TTS: Using CPU"); DeviceSpec::Cpu }
        };

        let options = RuntimeOptions::new(model_root).with_device(device);

        match Phase3Pipeline::from_options(options) {
            Ok(pipeline) => {
                info!("OmniVoice TTS pipeline loaded successfully.");
                Ok(Self {
                    pipeline: Mutex::new(Some(pipeline)),
                    loaded: true,
                    sample_rate: 24000,
                })
            }
            Err(e) => {
                warn!("Failed to load OmniVoice pipeline: {}. TTS will use placeholder audio.", e);
                Ok(Self {
                    pipeline: Mutex::new(None),
                    loaded: false,
                    sample_rate: 24000,
                })
            }
        }
    }

    #[cfg(not(feature = "cuda"))]
    pub fn new(models_path: &str) -> Result<Self> {
        warn!("TTS requires the 'cuda' feature. Building without TTS support.");
        let _ = models_path;
        Ok(Self {
            _unused: (),
            loaded: false,
            sample_rate: 24000,
        })
    }

    #[cfg(feature = "cuda")]
    pub fn synthesize(&self, text: &str, ref_audio: Option<Vec<f32>>, ref_text: Option<&str>) -> Result<Vec<f32>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        if !self.loaded {
            info!("TTS model not loaded, returning placeholder audio");
            let word_count = text.split_whitespace().count();
            let duration_secs = (word_count as f32 * 0.15).max(1.0);
            let num_samples = (self.sample_rate as f32 * duration_secs) as usize;
            return Ok(self.generate_placeholder_audio(num_samples));
        }

        // Convert numbers to words for better TTS
        let text_with_words = Self::numbers_to_words(text);

        info!("Synthesizing {} chars via OmniVoice...", text_with_words.len());

        let ref_audio_path = if let Some(ref_samples) = ref_audio {
            if ref_samples.len() > 16000 {
                if let Some(path) = self.save_ref_audio(&ref_samples) {
                    Some(path)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let req = if let Some(ref_path) = ref_audio_path {
            info!("Using voice cloning with ref audio: {}", ref_path);
            let mut req = GenerationRequest::new_text_only(text_with_words.as_str())
                .with_ref_audio(ReferenceAudioInput::from_path(ref_path));
            if let Some(rt) = ref_text {
                req = req.with_ref_text(rt);
                info!("Using ref_text for better voice cloning: {}", rt);
            }
            req
        } else {
            GenerationRequest::new_text_only(text_with_words.as_str())
        };

        let mut pipeline_lock = self.pipeline.lock().unwrap();
        if let Some(pipeline) = pipeline_lock.as_mut() {
            match pipeline.generate(&req) {
                Ok(audio_results) => {
                    if let Some(audio) = audio_results.into_iter().next() {
                        info!("OmniVoice synthesis complete: {} samples", audio.samples.len());
                        return Ok(audio.samples);
                    }
                }
                Err(e) => {
                    warn!("OmniVoice synthesis error: {}", e);
                }
            }
        }

        let word_count = text_with_words.split_whitespace().count();
        let duration_secs = (word_count as f32 * 0.15).max(1.0);
        let num_samples = (self.sample_rate as f32 * duration_secs) as usize;
        Ok(self.generate_placeholder_audio(num_samples))
    }

    pub fn numbers_to_words(text: &str) -> String {
        let re = Regex::new(r"\d+").unwrap();
        let mut result = text.to_string();
        
        for cap in re.find_iter(text) {
            if let Ok(num) = cap.as_str().parse::<u64>() {
                let words = Self::number_to_words_english(num);
                result = result.replace(cap.as_str(), &words);
            }
        }
        result
    }

    pub fn number_to_words_english(n: u64) -> String {
        if n == 0 {
            return "zero".to_string();
        }
        
        let units = ["", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];
        let teens = ["ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen"];
        let tens = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
        
        fn write(n: u64, units: &[&str], teens: &[&str], tens: &[&str]) -> String {
            if n == 0 {
                String::new()
            } else if n < 10 {
                units[n as usize].to_string()
            } else if n < 20 {
                teens[(n - 10) as usize].to_string()
            } else if n < 100 {
                let ten = (n / 10) as usize;
                let unit = (n % 10) as usize;
                if unit == 0 {
                    tens[ten].to_string()
                } else {
                    format!("{}-{}", tens[ten], units[unit])
                }
            } else if n < 1000 {
                let hundred = n / 100;
                let remainder = n % 100;
                if remainder == 0 {
                    format!("{} hundred", units[hundred as usize])
                } else {
                    format!("{} hundred {}", units[hundred as usize], write(remainder, units, teens, tens))
                }
            } else if n < 1_000_000 {
                let thousand = n / 1000;
                let remainder = n % 1000;
                if remainder == 0 {
                    format!("{} thousand", write(thousand, units, teens, tens))
                } else {
                    format!("{} thousand {}", write(thousand, units, teens, tens), write(remainder, units, teens, tens))
                }
            } else if n < 1_000_000_000 {
                let million = n / 1_000_000;
                let remainder = n % 1_000_000;
                if remainder == 0 {
                    format!("{} million", write(million, units, teens, tens))
                } else {
                    format!("{} million {}", write(million, units, teens, tens), write(remainder, units, teens, tens))
                }
            } else {
                let billion = n / 1_000_000_000;
                let remainder = n % 1_000_000_000;
                if remainder == 0 {
                    format!("{} billion", write(billion, units, teens, tens))
                } else {
                    format!("{} billion {}", write(billion, units, teens, tens), write(remainder, units, teens, tens))
                }
            }
        }
        
        write(n, &units, &teens, &tens)
    }

    #[cfg(feature = "cuda")]
    fn save_ref_audio(&self, samples: &[f32]) -> Option<String> {
        let temp_dir = std::env::temp_dir();
        let ref_path = temp_dir.join("voicy_ref_audio.wav");

        let sample_rate: u32 = 24000;
        let data_size = samples.len() * 4;
        let file_size = 36 + data_size as u32;

        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"RIFF");
        buffer.extend_from_slice(&file_size.to_le_bytes());
        buffer.extend_from_slice(b"WAVE");

        buffer.extend_from_slice(b"fmt ");
        buffer.extend_from_slice(&16u32.to_le_bytes());
        buffer.extend_from_slice(&3u16.to_le_bytes());
        buffer.extend_from_slice(&1u16.to_le_bytes());
        buffer.extend_from_slice(&sample_rate.to_le_bytes());
        buffer.extend_from_slice(&(sample_rate * 4).to_le_bytes());
        buffer.extend_from_slice(&4u16.to_le_bytes());
        buffer.extend_from_slice(&32u16.to_le_bytes());

        buffer.extend_from_slice(b"data");
        buffer.extend_from_slice(&(data_size as u32).to_le_bytes());

        for &sample in samples {
            buffer.extend_from_slice(&sample.to_le_bytes());
        }

        if let Ok(mut file) = fs::File::create(&ref_path) {
            if file.write_all(&buffer).is_ok() {
                return ref_path.to_str().map(|s| s.to_string());
            }
        }
        None
    }

    #[cfg(not(feature = "cuda"))]
    pub fn synthesize(&self, text: &str, _ref_audio: Option<Vec<f32>>, _ref_text: Option<&str>) -> Result<Vec<f32>> {
        let word_count = text.split_whitespace().count();
        let duration_secs = (word_count as f32 * 0.15).max(1.0);
        let num_samples = (self.sample_rate as f32 * duration_secs) as usize;
        Ok(self.generate_placeholder_audio(num_samples))
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