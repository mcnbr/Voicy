use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use std::sync::atomic::AtomicU32;

static CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static INPUT_SAMPLE_RATE: AtomicU32 = AtomicU32::new(48000);

// Persistent audio buffer — accumulates ALL samples during recording
static AUDIO_BUFFER: Mutex<Option<Arc<Mutex<Vec<f32>>>>> = Mutex::new(None);

// Rolling RMS level for UI visualizer (32 bins)
static AUDIO_LEVEL: Mutex<[f32; 32]> = Mutex::new([0.0f32; 32]);

pub fn init_audio_buffer() {
    let mut guard = AUDIO_BUFFER.lock().unwrap();
    if guard.is_none() {
        *guard = Some(Arc::new(Mutex::new(Vec::new())));
    }
}

pub fn clear_audio_buffer() {
    if let Ok(guard) = AUDIO_BUFFER.lock() {
        if let Some(buffer) = guard.as_ref() {
            if let Ok(mut buf) = buffer.lock() {
                buf.clear();
            }
        }
    }
}

pub fn get_audio_buffer_samples() -> usize {
    if let Ok(guard) = AUDIO_BUFFER.lock() {
        if let Some(buffer) = guard.as_ref() {
            if let Ok(buf) = buffer.lock() {
                return buf.len();
            }
        }
    }
    0
}

fn resample_linear(input: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
    if in_rate == out_rate || input.is_empty() {
        return input.to_vec();
    }
    let ratio = in_rate as f32 / out_rate as f32;
    let out_len = (input.len() as f32 / ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let in_idx = i as f32 * ratio;
        let idx_floor = in_idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(input.len() - 1);
        let weight = in_idx - idx_floor as f32;
        let sample = input[idx_floor] * (1.0 - weight) + input[idx_ceil] * weight;
        out.push(sample);
    }
    out
}

pub fn get_audio_buffer() -> Vec<f32> {
    if let Ok(guard) = AUDIO_BUFFER.lock() {
        if let Some(buffer) = guard.as_ref() {
            if let Ok(buf) = buffer.lock() {
                let in_rate = INPUT_SAMPLE_RATE.load(Ordering::SeqCst);
                return resample_linear(&buf, in_rate, 16000);
            }
        }
    }
    Vec::new()
}

pub fn get_last_recording_for_tts() -> Option<Vec<f32>> {
    if let Ok(guard) = AUDIO_BUFFER.lock() {
        if let Some(buffer) = guard.as_ref() {
            if let Ok(buf) = buffer.lock() {
                if buf.len() > 16000 {
                    let in_rate = INPUT_SAMPLE_RATE.load(Ordering::SeqCst);
                    let samples_24k = resample_linear(&buf, in_rate, 24000);
                    return Some(samples_24k);
                }
            }
        }
    }
    None
}

pub fn get_audio_levels() -> [f32; 32] {
    if let Ok(lvl) = AUDIO_LEVEL.lock() {
        *lvl
    } else {
        [0.0f32; 32]
    }
}

fn compute_rms_bins(data: &[f32], bins: &mut [f32; 32]) {
    if data.is_empty() {
        bins.fill(0.0);
        return;
    }
    let bin_size = (data.len() / 32).max(1);
    // Increase sensitivity factor from 1.0 to 3.0 for more visible bars
    let sensitivity = 3.0;
    for (i, bin) in bins.iter_mut().enumerate() {
        let start = i * bin_size;
        let end = ((i + 1) * bin_size).min(data.len());
        if start >= data.len() {
            *bin = 0.0;
            continue;
        }
        let rms = (data[start..end].iter().map(|s| s * s).sum::<f32>() / (end - start) as f32).sqrt() * sensitivity;
        // Smooth towards new value with faster response
        *bin = (*bin * 0.5 + rms * 0.5).min(1.0);
    }
}

pub fn start_capture_thread() -> Result<(), String> {
    if CAPTURE_ACTIVE.load(Ordering::SeqCst) {
        return Err("Capture already active".to_string());
    }

    let host = cpal::default_host();
    
    let device = host.default_input_device()
        .ok_or_else(|| "No default input device found".to_string())?;
        
    info!("Found audio input device: {:?}", device.name().unwrap_or_default());
    
    let config = device.default_input_config()
        .map_err(|e| format!("Failed to get default input config: {}", e))?;
        
    info!("Audio config: {} Hz, {} channels", config.sample_rate().0, config.channels());
    
    INPUT_SAMPLE_RATE.store(config.sample_rate().0, Ordering::SeqCst);
    
    let channels = config.channels() as usize;
    let sample_rate = config.sample_rate().0;
    info!("Audio config: {} Hz, {} channels", sample_rate, channels);

    let buffer = {
        let guard = AUDIO_BUFFER.lock().unwrap();
        guard.as_ref().map(|b| b.clone())
    };

    let Some(buffer) = buffer else {
        return Err("Audio buffer not initialized".to_string());
    };

    CAPTURE_ACTIVE.store(true, Ordering::SeqCst);
    clear_audio_buffer();

    // Reset level meters
    if let Ok(mut lvl) = AUDIO_LEVEL.lock() {
        lvl.fill(0.0);
    }

    thread::spawn(move || {
        let err_fn = |err| error!("Audio capture error: {}", err);

        // Helper closure: converts multi-channel data to mono f32 and accumulates
        let process_samples = {
            let buffer = buffer.clone();
            move |mono_data: Vec<f32>| {
                // Update RMS level visualizer
                if let Ok(mut lvl) = AUDIO_LEVEL.lock() {
                    compute_rms_bins(&mono_data, &mut lvl);
                }
                // Accumulate into main buffer (no clearing during capture!)
                if let Ok(mut buf) = buffer.lock() {
                    buf.extend_from_slice(&mono_data);
                }
            }
        };

        let process_samples = Arc::new(Mutex::new(process_samples));

        let result = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let ps = process_samples.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        if CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                            // Mix down to mono
                            let mono: Vec<f32> = data
                                .chunks(channels)
                                .map(|ch| ch.iter().sum::<f32>() / channels as f32)
                                .collect();
                            if let Ok(f) = ps.lock() { f(mono); }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let ps = process_samples.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &_| {
                        if CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                            let mono: Vec<f32> = data
                                .chunks(channels)
                                .map(|ch| ch.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32)
                                .collect();
                            if let Ok(f) = ps.lock() { f(mono); }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                let ps = process_samples.clone();
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &_| {
                        if CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                            let mono: Vec<f32> = data
                                .chunks(channels)
                                .map(|ch| ch.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).sum::<f32>() / channels as f32)
                                .collect();
                            if let Ok(f) = ps.lock() { f(mono); }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => {
                error!("Unsupported sample format");
                CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
                return;
            }
        };

        match result {
            Ok(stream) => {
                if let Err(e) = stream.play() {
                    error!("Failed to start stream: {}", e);
                    CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
                    return;
                }
                info!("Audio stream started successfully");

                while CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(50));
                }

                drop(stream);
                // Reset levels when capture stops
                if let Ok(mut lvl) = AUDIO_LEVEL.lock() {
                    lvl.fill(0.0);
                }
                info!("Audio stream stopped. Total samples: {}", get_audio_buffer_samples());
            }
            Err(e) => {
                error!("Failed to build stream: {}", e);
                CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
            }
        }

        CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
        info!("Capture thread exiting");
    });

    Ok(())
}

pub fn stop_capture_thread() -> Result<usize, String> {
    if !CAPTURE_ACTIVE.load(Ordering::SeqCst) {
        return Err("Capture not active".to_string());
    }

    CAPTURE_ACTIVE.store(false, Ordering::SeqCst);
    // Give the thread time to flush and stop
    thread::sleep(Duration::from_millis(300));

    let samples = get_audio_buffer_samples();
    info!("Capture stopped. Total samples: {}", samples);
    Ok(samples)
}

pub fn is_capturing() -> bool {
    CAPTURE_ACTIVE.load(Ordering::SeqCst)
}

pub fn test_audio_input() -> Result<bool, String> {
    let host = cpal::default_host();
    match host.default_input_device() {
        Some(device) => match device.name() {
            Ok(name) => {
                info!("Found audio input device: {}", name);
                Ok(true)
            }
            Err(_) => Ok(false),
        },
        None => Ok(false),
    }
}

pub fn get_audio_devices() -> Result<(Vec<String>, Vec<String>), String> {
    let host = cpal::default_host();
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    if let Ok(devices) = host.input_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                inputs.push(name);
            }
        }
    }
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            if let Ok(name) = d.name() {
                outputs.push(name);
            }
        }
    }
    Ok((inputs, outputs))
}