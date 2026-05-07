const API_BASE = "http://127.0.0.1:8765";

export interface AudioDevice {
  id: number;
  name: string;
  channels: number;
  sample_rate: number;
  hostapi: number;
}

export interface DevicesResponse {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  default_input: number | null;
  default_output: number | null;
}

export interface AppConfig {
  input_device: number | null;
  output_device: number | null;
  source_lang: string;
  target_lang: string;
  routing_mode: string;
  ref_audio_path: string | null;
  supported_languages: Record<string, string>;
}

export interface ModelsStatusResponse {
  whisper: string;
  gemma: string;
  omnivoice: string;
  omnivoice_has_ref: boolean;
}

export async function fetchDevices(): Promise<DevicesResponse> {
  const res = await fetch(`${API_BASE}/api/devices`);
  return res.json();
}

export async function fetchConfig(): Promise<AppConfig> {
  const res = await fetch(`${API_BASE}/api/config`);
  return res.json();
}

export async function updateConfig(update: Partial<AppConfig>): Promise<any> {
  const res = await fetch(`${API_BASE}/api/config`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(update),
  });
  return res.json();
}

export async function fetchModelsStatus(): Promise<ModelsStatusResponse> {
  const res = await fetch(`${API_BASE}/api/models/status`);
  return res.json();
}

export async function uploadRefAudio(file: File): Promise<{ status: string; path: string }> {
  const formData = new FormData();
  formData.append("file", file);
  const res = await fetch(`${API_BASE}/api/ref-audio`, {
    method: "POST",
    body: formData,
  });
  return res.json();
}

export async function fetchHealth(): Promise<any> {
  const res = await fetch(`${API_BASE}/health`);
  return res.json();
}
