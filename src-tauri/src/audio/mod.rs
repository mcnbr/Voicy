use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, SampleFormat, StreamConfig};
use log::info;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;

pub struct AudioCapture {
    host: Host,
    input_device: Option<Device>,
    stream: Option<cpal::Stream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    is_running: Arc<AtomicBool>,
}

impl AudioCapture {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let is_running = Arc::new(AtomicBool::new(false));
        Self {
            host,
            input_device: None,
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            is_running,
        }
    }

    pub fn list_input_devices(&self) -> Vec<String> {
        match self.host.input_devices() {
            Ok(devices) => devices.filter_map(|d| d.name().ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn list_output_devices(&self) -> Vec<String> {
        match self.host.output_devices() {
            Ok(devices) => devices.filter_map(|d| d.name().ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn set_input_device(&mut self, device_name: &str) -> Result<(), String> {
        let device = self
            .host
            .input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
            .ok_or_else(|| format!("Device not found: {}", device_name))?;

        self.input_device = Some(device);
        info!("Input device set: {}", device_name);
        Ok(())
    }

    pub fn set_output_device(&mut self, device_name: &str) -> Result<(), String> {
        let device = self
            .host
            .output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
            .ok_or_else(|| format!("Device not found: {}", device_name))?;

        info!("Output device set: {}", device_name);
        Ok(())
    }

    pub fn start_capture(&mut self) -> Result<(), String> {
        let device = self.input_device.as_ref().ok_or("No input device selected")?;

        let config = device
            .default_input_config()
            .map_err(|e| e.to_string())?;

        info!("Input config: {:?}", config);

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        let buffer = self.buffer.clone();
        let is_running = self.is_running.clone();

        let stream = match config.sample_format() {
            SampleFormat::F32 => device.build_input_stream(
                &StreamConfig {
                    sample_rate: cpal::SampleRate(sample_rate),
                    channels,
                    buffer_size: cpal::BufferSize::Default,
                },
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if is_running.load(Ordering::SeqCst) {
                        let mut buf = buffer.lock();
                        buf.extend_from_slice(data);
                    }
                },
                |err| log::error!("Input stream error: {}", err),
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                &StreamConfig {
                    sample_rate: cpal::SampleRate(sample_rate),
                    channels,
                    buffer_size: cpal::BufferSize::Default,
                },
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if is_running.load(Ordering::SeqCst) {
                        let mut buf = buffer.lock();
                        for &sample in data {
                            buf.push(sample as f32 / i16::MAX as f32);
                        }
                    }
                },
                |err| log::error!("Input stream error: {}", err),
                None,
            ),
            _ => return Err("Unsupported sample format".to_string()),
        }
        .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);
        self.is_running.store(true, Ordering::SeqCst);

        info!("Capture started");
        Ok(())
    }

    pub fn stop_capture(&mut self) -> Result<Vec<f32>, String> {
        self.is_running.store(false, Ordering::SeqCst);
        self.stream = None;

        let mut buffer = self.buffer.lock();
        let audio = buffer.clone();
        buffer.clear();

        Ok(audio)
    }

    pub fn get_buffer(&self) -> Vec<f32> {
        self.buffer.lock().clone()
    }

    pub fn clear_buffer(&self) {
        self.buffer.lock().clear();
    }
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AudioPlayback {
    host: Host,
    output_device: Option<Device>,
    stream: Option<cpal::Stream>,
}

impl AudioPlayback {
    pub fn new() -> Self {
        let host = cpal::default_host();
        Self {
            host,
            output_device: None,
            stream: None,
        }
    }

    pub fn set_device(&mut self, device_name: &str) -> Result<(), String> {
        let device = self
            .host
            .output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().map(|n| n == device_name).unwrap_or(false))
            .ok_or_else(|| format!("Device not found: {}", device_name))?;

        self.output_device = Some(device);
        info!("Output device set: {}", device_name);
        Ok(())
    }

    pub fn play(&mut self, audio_data: Vec<f32>, sample_rate: u32, channels: u16) -> Result<(), String> {
        let device = self.output_device.as_ref().ok_or("No output device selected")?;

        let _config = device
            .default_output_config()
            .map_err(|e| e.to_string())?;

        let audio = audio_data;
        let stream = device
            .build_output_stream(
                &StreamConfig {
                    sample_rate: cpal::SampleRate(sample_rate),
                    channels,
                    buffer_size: cpal::BufferSize::Default,
                },
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let len = audio.len().min(data.len());
                    data[..len].copy_from_slice(&audio[..len]);
                    if audio.len() < data.len() {
                        data[audio.len()..].fill(0.0);
                    }
                },
                |err| log::error!("Output stream error: {}", err),
                None,
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);

        info!("Playback started");
        Ok(())
    }

    pub fn stop(&mut self) {
        self.stream = None;
    }
}

impl Default for AudioPlayback {
    fn default() -> Self {
        Self::new()
    }
}