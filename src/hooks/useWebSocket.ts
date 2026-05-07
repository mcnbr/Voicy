import { useEffect, useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export interface PipelineTelemetry {
  stage: "stt" | "translate" | "tts";
  status: "running" | "done" | "error";
  duration_ms: number;
}

export interface ModelsStatus {
  whisper: string;
  gemma: string;
  omnivoice: string;
}

export interface AudioDevice {
  id: number;
  name: string;
  channels: number;
  sample_rate: number;
  hostapi: number;
}

export interface DevicesData {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
  default_input: number | null;
  default_output: number | null;
}

export interface ConfigData {
  input_device: number | null;
  output_device: number | null;
  source_lang: string;
  target_lang: string;
  routing_mode: string;
  ref_audio_path: string | null;
  supported_languages: Record<string, string>;
  live_threshold: number;
  live_silence_duration: number;
}

export interface TranscriptionData {
  text: string;
  source_lang: string;
}

export interface TranslationData {
  text: string;
  target_lang: string;
}

export interface CalibrationStatus {
  status: "idle" | "started" | "complete" | "error";
  threshold?: number;
  error?: string;
}

interface UseWebSocketReturn {
  isConnected: boolean;
  isRecording: boolean;
  pipelineTelemetry: PipelineTelemetry[];
  modelsStatus: ModelsStatus;
  lastTranscription: TranscriptionData | null;
  lastTranslation: TranslationData | null;
  audioLevel: number[] | null;
  refAudioStatus: { used_voice_cloning: boolean; ref_source: string | null } | null;
  manualTriggerStatus: string | null;
  liveState: string | null;
  calibrationStatus: CalibrationStatus;
  calibratedThreshold: number | null;
  devices: DevicesData | null;
  config: ConfigData | null;
  send: (msg: any) => void;
  updateConfig: (data: Partial<ConfigData>) => void;
  playLast: () => void;
  toggleRecording: () => void;
  calibrateThreshold: () => void;
}

export function useWebSocket(): UseWebSocketReturn {
  const [isConnected, setIsConnected] = useState(true);
  const [isRecording, setIsRecording] = useState(false);
  const [pipelineTelemetry, setPipelineTelemetry] = useState<PipelineTelemetry[]>([]);
  const [modelsStatus, setModelsStatus] = useState<ModelsStatus>({
    whisper: "loading",
    gemma: "loading",
    omnivoice: "loading",
  });
  const [lastTranscription, setLastTranscription] = useState<TranscriptionData | null>(null);
  const [lastTranslation, setLastTranslation] = useState<TranslationData | null>(null);
  const [audioLevel] = useState<number[] | null>(null);
  const [refAudioStatus] = useState<{ used_voice_cloning: boolean; ref_source: string | null } | null>(null);
  const [manualTriggerStatus] = useState<string | null>(null);
  const [liveState] = useState<string | null>(null);
  const [calibrationStatus] = useState<CalibrationStatus>({ status: "idle" });
  const [calibratedThreshold] = useState<number | null>(null);
  const [devices, setDevices] = useState<DevicesData | null>(null);
  const [config, setConfig] = useState<ConfigData | null>(null);

  useEffect(() => {
    // Load config and devices initially
    invoke("get_config").then((cfg: any) => {
      setConfig({
        input_device: null,
        output_device: null,
        source_lang: "auto",
        target_lang: "en",
        routing_mode: cfg.mode || "auto",
        ref_audio_path: null,
        supported_languages: {
            "en": "English", "pt": "Portuguese", "es": "Spanish"
        },
        live_threshold: -50.0,
        live_silence_duration: 2.0
      });
    }).catch(console.error);

    invoke("list_audio_devices").then((devs: any) => {
      setDevices({
        inputs: devs.inputs.map((d: string, i: number) => ({ id: i, name: d, channels: 1, sample_rate: 16000, hostapi: 0 })),
        outputs: devs.outputs.map((d: string, i: number) => ({ id: i, name: d, channels: 1, sample_rate: 16000, hostapi: 0 })),
        default_input: 0,
        default_output: 0
      });
    }).catch(console.error);
    
    invoke("check_models_status").then((status: any) => {
        if (status === "ready") {
            setModelsStatus({ whisper: "ready", gemma: "ready", omnivoice: "ready" });
        }
    }).catch(console.error);
  }, []);

  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const res: any = await invoke("get_status");
        setIsConnected(true);
        setIsRecording(res.status === "recording");
        
        if (res.last_transcription) {
          setLastTranscription({ text: res.last_transcription, source_lang: "auto" });
        }
        if (res.last_translation) {
          setLastTranslation({ text: res.last_translation, target_lang: "auto" });
        }
        
        if (res.status === "loading") {
            setModelsStatus({ whisper: "loading", gemma: "loading", omnivoice: "loading" });
        } else if (res.status === "ready" || res.status === "recording" || res.status === "processing") {
            setModelsStatus({ whisper: "ready", gemma: "ready", omnivoice: "ready" });
        }
        
        if (res.status === "processing") {
            setPipelineTelemetry([{ stage: "stt", status: "running", duration_ms: 0 }]);
        } else if (res.status === "ready" && res.last_translation) {
            setPipelineTelemetry([
                { stage: "stt", status: "done", duration_ms: 500 },
                { stage: "translate", status: "done", duration_ms: 500 },
                { stage: "tts", status: "done", duration_ms: 500 }
            ]);
        }
      } catch (e) {
        setIsConnected(false);
      }
    }, 500);
    return () => clearInterval(interval);
  }, []);

  const send = useCallback((_msg: any) => {
    // legacy send compatibility
  }, []);

  const updateConfig = useCallback(async (data: Partial<ConfigData>) => {
    if (data.routing_mode) {
      await invoke("set_mode", { mode: data.routing_mode }).catch(console.error);
    }
    if (data.source_lang) {
      await invoke("set_source_language", { language: data.source_lang }).catch(console.error);
    }
    if (data.target_lang) {
      await invoke("set_target_language", { language: data.target_lang }).catch(console.error);
    }
    setConfig(prev => prev ? { ...prev, ...data } : null);
  }, []);

  const playLast = useCallback(() => {
    // not implemented
  }, []);

  const toggleRecording = useCallback(async () => {
    if (isRecording) {
      await invoke("stop_capture").catch(console.error);
      setIsRecording(false);
    } else {
      await invoke("start_capture").catch(console.error);
      setIsRecording(true);
    }
  }, [isRecording]);

  const calibrateThreshold = useCallback(() => {
    // not implemented
  }, []);

  return {
    isConnected,
    isRecording,
    pipelineTelemetry,
    modelsStatus,
    lastTranscription,
    lastTranslation,
    audioLevel,
    refAudioStatus,
    manualTriggerStatus,
    liveState,
    calibrationStatus,
    calibratedThreshold,
    devices,
    config,
    send,
    updateConfig,
    playLast,
    toggleRecording,
    calibrateThreshold,
  };
}
