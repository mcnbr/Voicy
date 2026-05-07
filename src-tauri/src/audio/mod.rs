use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use log::{error, info};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

static CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
static AUDIO_BUFFER: Mutex<Option<Arc<Mutex<Vec<f32>>>>> = Mutex::new(None);

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

pub fn get_audio_buffer() -> Vec<f32> {
    if let Ok(guard) = AUDIO_BUFFER.lock() {
        if let Some(buffer) = guard.as_ref() {
            if let Ok(buf) = buffer.lock() {
                return buf.clone();
            }
        }
    }
    Vec::new()
}

pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    if let Ok(enumerator) = host.input_devices() {
        for d in enumerator {
            if let Ok(name) = d.name() {
                devices.push(name);
            }
        }
    }
    devices
}

pub fn list_output_devices() -> Vec<String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();
    if let Ok(enumerator) = host.output_devices() {
        for d in enumerator {
            if let Ok(name) = d.name() {
                devices.push(name);
            }
        }
    }
    devices
}

pub fn start_capture_thread() -> Result<(), String> {
    if CAPTURE_ACTIVE.load(Ordering::SeqCst) {
        return Err("Capture already active".to_string());
    }

    let host = cpal::default_host();
    let device = match host.default_input_device() {
        Some(d) => d,
        None => return Err("No input device available".to_string()),
    };

    info!("Using audio device: {:?}", device.name());

    let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to get default config: {}", e)),
    };

    let sample_rate = config.sample_rate().0;
    let channels = config.channels();
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

    let (_stop_tx, stop_rx) = std_mpsc::channel::<()>();

    thread::spawn(move || {
        let err_fn = |err| error!("Audio capture error: {}", err);
        let buffer = buffer;

        let result = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _: &_| {
                        if CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                            if let Ok(mut buf) = buffer.lock() {
                                buf.extend_from_slice(data);
                                if buf.len() >= 16000 {
                                    let chunk = buf.len();
                                    buf.clear();
                                    info!("Audio chunk captured: {} samples", chunk);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _: &_| {
                        if CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                            if let Ok(mut buf) = buffer.lock() {
                                let float_data: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                                buf.extend_from_slice(&float_data);
                                if buf.len() >= 16000 {
                                    let chunk = buf.len();
                                    buf.clear();
                                    info!("Audio chunk captured: {} samples", chunk);
                                }
                            }
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _: &_| {
                        if CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                            if let Ok(mut buf) = buffer.lock() {
                                let float_data: Vec<f32> = data.iter().map(|&s| (s as f32 - 32768.0) / 32768.0).collect();
                                buf.extend_from_slice(&float_data);
                                if buf.len() >= 16000 {
                                    let chunk = buf.len();
                                    buf.clear();
                                    info!("Audio chunk captured: {} samples", chunk);
                                }
                            }
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

                loop {
                    if stop_rx.try_recv().is_ok() {
                        info!("Stop signal received");
                        break;
                    }
                    if !CAPTURE_ACTIVE.load(Ordering::SeqCst) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }

                drop(stream);
                info!("Audio stream stopped");
            }
            Err(e) => {
                error!("Failed to build stream: {}", e);
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

    thread::sleep(Duration::from_millis(200));

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
        Some(device) => {
            match device.name() {
                Ok(name) => {
                    info!("Found audio input device: {}", name);
                    Ok(true)
                }
                Err(_) => Ok(false)
            }
        }
        None => Ok(false)
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

pub struct AudioPlayback {
    output_stream: Option<cpal::Stream>,
    #[allow(dead_code)]
    samples: Option<Vec<f32>>,
}

impl AudioPlayback {
    pub fn new() -> Self {
        Self {
            output_stream: None,
            samples: None,
        }
    }

    pub fn play(&mut self, samples: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No output device available")?;

        let config = device.default_output_config()?;

        let err_fn = |err| error!("Audio playback error: {}", err);

        let samples_owned: Vec<f32> = samples.to_vec();
        let samples_len = samples_owned.len();
        let samples_for_closure = samples_owned.clone();

        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &_| {
                let len = data.len().min(samples_len);
                data[..len].copy_from_slice(&samples_for_closure[..len]);
                if len < data.len() {
                    data[len..].fill(0.0);
                }
            },
            err_fn,
            None,
        )?;

        stream.play()?;
        self.output_stream = Some(stream);
        self.samples = Some(samples_owned);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.output_stream = None;
        self.samples = None;
    }
}

impl Default for AudioPlayback {
    fn default() -> Self {
        Self::new()
    }
}