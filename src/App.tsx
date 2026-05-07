import { useState, useEffect, useCallback, useRef } from 'react';
import { useWebSocket } from './hooks/useWebSocket';

function App() {
  const [outputMode, setOutputMode] = useState('auto');
  const [isApplying, setIsApplying] = useState(false);
  const [selectedInput, setSelectedInput] = useState<number | null>(null);
  const [selectedOutput, setSelectedOutput] = useState<number | null>(null);
  const [sourceLang, setSourceLang] = useState('en');
  const [targetLang, setTargetLang] = useState('en');
  const [modePresets, setModePresets] = useState<Record<string, any>>(() => {
    try {
      const saved = localStorage.getItem('voicy_mode_presets');
      return saved ? JSON.parse(saved) : {};
    } catch {
      return {};
    }
  });
  const [audioLevels, setAudioLevels] = useState<number[]>(new Array(32).fill(0));
  const [audioUrl, setAudioUrl] = useState<string | null>(null);
  const [isAudioPlaying, setIsAudioPlaying] = useState(false);
  const [audioDuration, setAudioDuration] = useState(0);
  const [audioCurrentTime, setAudioCurrentTime] = useState(0);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const [liveThresholdDb, setLiveThresholdDb] = useState(-50.0);
  const [liveSilenceDuration, setLiveSilenceDuration] = useState(2.0);
  const [calibrateCountdown, setCalibrateCountdown] = useState(0);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const animFrameRef = useRef<number>(0);
  const hiddenAudioRef = useRef<HTMLAudioElement | null>(null);

  const {
    isConnected,
    isRecording,
    pipelineTelemetry,
    modelsStatus,
    lastTranscription,
    lastTranslation,
    audioLevel,
    devices,
    config,
    updateConfig,
    toggleRecording,
    calibrationStatus,
    calibratedThreshold,
    calibrateThreshold,
  } = useWebSocket();

  const sttTime = pipelineTelemetry.find(t => t.stage === 'stt');
  const tlTime = pipelineTelemetry.find(t => t.stage === 'translate');
  const ttsTime = pipelineTelemetry.find(t => t.stage === 'tts');
  const isProcessing = sttTime?.status === 'running' || tlTime?.status === 'running' || ttsTime?.status === 'running';

  const allModelsReady = modelsStatus.whisper === 'ready' && modelsStatus.gemma === 'ready' && modelsStatus.omnivoice === 'ready';
  const anyModelError = modelsStatus.whisper === 'error' || modelsStatus.gemma === 'error' || modelsStatus.omnivoice === 'error';
  const modelsLoading = !allModelsReady && !anyModelError;

  const getStatusText = () => {
    if (!isConnected) return 'Disconnected';
    if (modelsLoading) return 'Loading Models...';
    if (isRecording) return 'Recording';
    if (isProcessing) return 'Processing...';
    return 'Connected';
  };

  const getStatusColor = () => {
    if (!isConnected) return '#ff453a';
    if (modelsLoading) return '#ffd60a';
    if (isRecording) return '#ff453a';
    if (isProcessing) return '#2ECA7F';
    return '#2ECA7F';
  };

  // Helper: convert RMS to dBFS
  const rmsToDb = (rms: number) => 20 * Math.log10(Math.max(rms, 1e-10));
  const dbToRms = (db: number) => Math.pow(10, db / 20);

  useEffect(() => {
    if (config) {
      setSelectedInput(config.input_device);
      setSelectedOutput(config.output_device);
      setSourceLang(config.source_lang === 'auto' ? 'en' : config.source_lang);
      setTargetLang(config.target_lang);
      setOutputMode(config.routing_mode);
      if (config.live_threshold !== undefined) setLiveThresholdDb(rmsToDb(config.live_threshold));
      if (config.live_silence_duration !== undefined) setLiveSilenceDuration(config.live_silence_duration);
    }
  }, [config?.input_device, config?.output_device, config?.source_lang, config?.target_lang, config?.routing_mode, config?.live_threshold, config?.live_silence_duration]);

  // Countdown timer for calibration
  useEffect(() => {
    if (calibrationStatus.status === 'started') {
      setCalibrateCountdown(5);
      const interval = setInterval(() => {
        setCalibrateCountdown((prev) => {
          if (prev <= 1) {
            clearInterval(interval);
            return 0;
          }
          return prev - 1;
        });
      }, 1000);
      return () => clearInterval(interval);
    }
  }, [calibrationStatus.status]);

  // Update slider when backend reports new calibrated threshold
  useEffect(() => {
    if (calibratedThreshold !== null) {
      setLiveThresholdDb(rmsToDb(calibratedThreshold));
    }
  }, [calibratedThreshold]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.code === 'Space' && e.ctrlKey && !e.repeat) {
        e.preventDefault();
        if (!isRecording && isConnected) {
          toggleRecording();
        }
      }
    };
    const handleKeyUp = (e: KeyboardEvent) => {
      if ((e.code === 'Space' || e.key === 'Control') && isRecording) {
        toggleRecording();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('keyup', handleKeyUp);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('keyup', handleKeyUp);
    };
  }, [isRecording, isConnected, toggleRecording]);

  useEffect(() => {
    if (audioLevel && audioLevel.length > 0) {
      setAudioLevels(audioLevel);
    } else if (!isRecording && outputMode !== 'live') {
      setAudioLevels(new Array(32).fill(0));
    } else if (!isRecording && outputMode === 'live' && audioLevel && audioLevel.every(v => v === 0)) {
      setAudioLevels(new Array(32).fill(0));
    }
  }, [audioLevel, isRecording, outputMode]);

  useEffect(() => {
    let animationFrameId: number;
    const updateProgress = () => {
      if (hiddenAudioRef.current && isAudioPlaying) {
        setAudioCurrentTime(hiddenAudioRef.current.currentTime);
        animationFrameId = requestAnimationFrame(updateProgress);
      }
    };
    if (isAudioPlaying) {
      animationFrameId = requestAnimationFrame(updateProgress);
    }
    return () => {
      if (animationFrameId) cancelAnimationFrame(animationFrameId);
    };
  }, [isAudioPlaying]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const draw = () => {
      const { width, height } = canvas;
      ctx.clearRect(0, 0, width, height);
      const barCount = audioLevels.length;
      const barWidth = Math.max(2, (width / barCount) - 2);
      const gap = 2;
      const centerY = height / 2;
      const accentGreen = '#2ECA7F';

      for (let i = 0; i < barCount; i++) {
        const level = audioLevels[i];
        const barHeight = Math.max(1, level * (height * 0.4));
        const x = i * (barWidth + gap);
        ctx.shadowColor = accentGreen;
        ctx.shadowBlur = isRecording ? 6 + level * 10 : 0;
        ctx.fillStyle = accentGreen;
        ctx.globalAlpha = isRecording ? 0.6 + level * 0.4 : 0.15;
        ctx.beginPath();
        ctx.roundRect(x, centerY - barHeight, barWidth, barHeight, 2);
        ctx.fill();
        ctx.globalAlpha = isRecording ? 0.3 + level * 0.3 : 0.08;
        ctx.beginPath();
        ctx.roundRect(x, centerY, barWidth, barHeight, 2);
        ctx.fill();
      }
      ctx.shadowBlur = 0;
      ctx.globalAlpha = 1;
      animFrameRef.current = requestAnimationFrame(draw);
    };
    draw();
    return () => cancelAnimationFrame(animFrameRef.current);
  }, [audioLevels, isRecording]);

  // Fetch last generated audio as blob URL when translation is done
  useEffect(() => {
    if (lastTranslation && ttsTime?.status === 'done' && ttsTime.duration_ms > 0) {
      fetch('http://127.0.0.1:8765/api/last-audio.wav')
        .then(res => res.blob())
        .then(blob => {
          const url = URL.createObjectURL(blob);
          setAudioUrl(url);
        })
        .catch(() => setAudioUrl(null));
    }
  }, [lastTranslation, ttsTime]);

  // Cleanup blob URL when it changes or on unmount
  useEffect(() => {
    return () => {
      if (audioUrl) URL.revokeObjectURL(audioUrl);
    };
  }, [audioUrl]);

  const savePreset = useCallback((mode: string) => {
    const preset = {
      input_device: selectedInput,
      output_device: selectedOutput,
      source_lang: sourceLang,
      target_lang: targetLang,
    };
    const next = { ...modePresets, [mode]: preset };
    setModePresets(next);
    localStorage.setItem('voicy_mode_presets', JSON.stringify(next));
  }, [selectedInput, selectedOutput, sourceLang, targetLang, modePresets]);

  const loadPreset = useCallback((mode: string) => {
    const preset = modePresets[mode];
    if (preset) {
      if (preset.input_device !== undefined) setSelectedInput(preset.input_device);
      if (preset.output_device !== undefined) setSelectedOutput(preset.output_device);
      if (preset.source_lang !== undefined) setSourceLang(preset.source_lang);
      if (preset.target_lang !== undefined) setTargetLang(preset.target_lang);
    }
  }, [modePresets]);

  const handleApply = useCallback(async () => {
    setIsApplying(true);
    savePreset(outputMode);
    updateConfig({
      input_device: selectedInput,
      output_device: selectedOutput,
      source_lang: sourceLang,
      target_lang: targetLang,
      routing_mode: outputMode,
    });
    setTimeout(() => setIsApplying(false), 800);
  }, [selectedInput, selectedOutput, sourceLang, targetLang, outputMode, updateConfig, savePreset]);

  const handleRoutingMode = useCallback((mode: string) => {
    // Save current mode preset before switching
    savePreset(outputMode);
    setOutputMode(mode);
    // Load preset for new mode
    loadPreset(mode);
    updateConfig({ routing_mode: mode });
  }, [updateConfig, outputMode, savePreset, loadPreset]);

  const handlePlayPause = useCallback(() => {
    if (!hiddenAudioRef.current || !audioUrl) return;
    if (isAudioPlaying) {
      hiddenAudioRef.current.pause();
    } else {
      hiddenAudioRef.current.play();
    }
  }, [isAudioPlaying, audioUrl]);

  const handleSeek = useCallback((e: React.MouseEvent<HTMLDivElement>) => {
    if (!hiddenAudioRef.current || !audioDuration) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const percent = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    hiddenAudioRef.current.currentTime = percent * audioDuration;
  }, [audioDuration]);

  const formatMs = (ms: number) => {
    if (ms === 0) return '—';
    return `${(ms / 1000).toFixed(2)}s`;
  };

  const supportedLanguages = config?.supported_languages ?? {
    en: 'English', pt: 'Portuguese',
    es: 'Spanish', fr: 'French', de: 'German', it: 'Italian',
    ja: 'Japanese', zh: 'Chinese', ko: 'Korean', ru: 'Russian',
  };

  return (
    <>
      <div className="card top-navbar" style={{ padding: '8px 16px', minHeight: 'unset', position: 'relative' }}>
        <img src="/voicy_logo.png" alt="Voicy" className="brand-logo-img" />
        <button
          className="gear-btn"
          onClick={() => setIsSettingsOpen(true)}
          style={{
            background: 'transparent',
            border: 'none',
            color: 'var(--text-secondary)',
            cursor: 'pointer',
            padding: '4px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.62 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.62a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
        </button>
      </div>

      <div className="grid-container">
        {/* Core Status Card */}
         <div className="card col-span-5" style={{ display: 'flex', flexDirection: 'column' }}>
           <div className="card-header">
             <span>System State</span>
           </div>
           <div className="value-large text-accent" style={{ color: getStatusColor(), fontSize: '1.75rem', marginBottom: '12px', display: 'flex', alignItems: 'center', gap: '10px' }}>
               {(modelsLoading || isProcessing) && (
                 <svg className="spinner" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" style={{ animation: 'spin 2s linear infinite' }}>
                   <path d="M21 12a9 9 0 1 1-6.219-8.56" />
                 </svg>
               )}
               {isRecording && (
                 <div style={{ width: '16px', height: '16px', borderRadius: '50%', backgroundColor: '#ff453a', animation: 'pulse 1.5s infinite' }} />
               )}
               {getStatusText()}
               <style>{`
                 @keyframes spin { 100% { transform: rotate(360deg); } }
                 @keyframes pulse { 0% { opacity: 1; } 50% { opacity: 0.4; } 100% { opacity: 1; } }
               `}</style>
           </div>

           <button
              className={`action-btn ${isRecording ? 'recording' : ''}`}
              onClick={() => toggleRecording()}
              style={{ marginBottom: '12px', width: '100%' }}
              disabled={!isConnected || modelsLoading}
           >
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                 <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/>
                 <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
                 <line x1="12" x2="12" y1="19" y2="22"/>
              </svg>
              {!isConnected
                ? 'Backend Not Connected'
                : modelsLoading
                  ? 'Loading Models...'
                  : isRecording
                    ? 'Stop Recording'
                    : 'Click to Record (Ctrl+Space)'}
           </button>

            <div style={{ marginTop: 'auto', paddingTop: '8px' }}>
              <canvas ref={canvasRef} width={400} height={60} style={{ width: '100%', height: '60px', borderRadius: '6px' }} />
            </div>
         </div>

        {/* Pipeline Configuration */}
        <div className="card col-span-7">
          <div className="card-header">
            <span>Configuration</span>
          </div>
          
          <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
            <div className="form-group" style={{ paddingBottom: '0' }}>
              <label style={{ minWidth: '70px' }}>Input</label>
              <select className="select-dropdown" style={{ flex: 1 }}
                value={selectedInput ?? ''} onChange={e => setSelectedInput(Number(e.target.value))}>
               {(devices?.inputs ?? []).map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
               {!devices && <option value="">Loading...</option>}
              </select>
            </div>
            
            <div className="form-group" style={{ paddingBottom: '0' }}>
              <label style={{ minWidth: '70px' }}>Source</label>
              <select className="select-dropdown" style={{ flex: 1 }}
                value={sourceLang} onChange={e => setSourceLang(e.target.value)}>
               {Object.entries(supportedLanguages).filter(([code]) => code !== 'auto').map(([code, name]) => <option key={code} value={code}>{name}</option>)}
              </select>
            </div>
            
            <div className="form-group" style={{ paddingBottom: '0' }}>
              <label style={{ minWidth: '70px' }}>Target</label>
              <select className="select-dropdown" style={{ flex: 1 }}
                value={targetLang} onChange={e => setTargetLang(e.target.value)}>
               {Object.entries(supportedLanguages).filter(([code]) => code !== 'auto').map(([code, name]) => <option key={code} value={code}>{name}</option>)}
              </select>
            </div>
            
              <div className="form-group" style={{ paddingBottom: '0' }}>
                <label style={{ minWidth: '70px' }}>Output</label>
                <select className="select-dropdown" style={{ flex: 1 }}
                  value={selectedOutput ?? ''} onChange={e => setSelectedOutput(Number(e.target.value))}>
                 {(devices?.outputs ?? []).map(d => <option key={d.id} value={d.id}>{d.name}</option>)}
                 {!devices && <option value="">Loading...</option>}
                </select>
              </div>

             <div className="form-group" style={{ paddingBottom: '0', marginTop: '4px' }}>
               <label style={{ minWidth: '70px' }}>Mode</label>
               <div className="toggle-group" style={{ flex: 1 }}>
                  <button className={`toggle-btn ${outputMode === 'auto' ? 'active' : ''}`} onClick={() => handleRoutingMode('auto')}>Auto</button>
                  <button className={`toggle-btn ${outputMode === 'live' ? 'active' : ''}`} onClick={() => handleRoutingMode('live')}>Live</button>
                  <button className={`toggle-btn ${outputMode === 'manual' ? 'active' : ''}`} onClick={() => handleRoutingMode('manual')}>Manual</button>
               </div>
             </div>
             
             <button className="apply-btn" onClick={handleApply} style={{ marginTop: '8px' }}>
               {isApplying ? 'Applying...' : 'Apply Configuration'}
             </button>
          </div>
        </div>

        {/* Telemetry - Models side by side */}
         <div className="card col-span-12" style={{ overflow: 'visible' }}>
            <div className="card-header">
               <span>Model Processing</span>
            </div>
           
           <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '12px' }}>
             
{/* Whisper STT */}
              <div>
                <div className="telemetry-item" style={{ borderRadius: '8px 8px 0 0', padding: '6px 10px' }}>
                   <span className="t-label">Whisper V3</span>
                   <span className="t-time" style={{ color: sttTime?.status === 'running' ? '#ffd60a' : (sttTime?.status === 'error' ? '#ff453a' : undefined) }}>
                      {sttTime ? formatMs(sttTime.duration_ms) : '—'}
                   </span>
                </div>
                {lastTranscription && (
                  <div style={{ 
                    padding: '8px 10px', 
                    background: 'var(--bg-color)', 
                    border: '1px solid var(--card-border)', 
                    borderTop: 'none',
                    borderRadius: '0 0 8px 8px',
                    fontSize: '0.8rem',
                    color: 'var(--text-primary)',
                    minHeight: '68px',
                    display: 'flex',
                    alignItems: 'center'
                  }}>
                    {lastTranscription.text}
                  </div>
                )}
              </div>

             {/* TranslateGemma TL */}
              <div>
                <div className="telemetry-item" style={{ borderRadius: '8px 8px 0 0', padding: '6px 10px' }}>
                   <span className="t-label">Translate Gemma</span>
                   <span className="t-time" style={{ color: tlTime?.status === 'running' ? '#ffd60a' : (tlTime?.status === 'error' ? '#ff453a' : undefined) }}>
                     {tlTime ? formatMs(tlTime.duration_ms) : '—'}
                   </span>
                </div>
                {lastTranslation && (
                  <div style={{ 
                    padding: '8px 10px', 
                    background: 'var(--bg-color)', 
                    border: '1px solid var(--card-border)', 
                    borderTop: 'none',
                    borderRadius: '0 0 8px 8px',
                    fontSize: '0.8rem',
                    color: '#2ECA7F',
                    minHeight: '68px',
                    display: 'flex',
                    alignItems: 'center'
                  }}>
                    {lastTranslation.text}
                  </div>
                )}
              </div>

{/* OmniVoice TTS with player */}
               <div>
                 <div className="telemetry-item" style={{ borderRadius: '8px 8px 0 0', padding: '6px 10px' }}>
                    <span className="t-label">Omnivoice</span>
                    <span className="t-time" style={{ color: ttsTime?.status === 'running' ? '#ffd60a' : (ttsTime?.status === 'error' ? '#ff453a' : undefined) }}>
                       {ttsTime ? formatMs(ttsTime.duration_ms) : '—'}
                    </span>
                 </div>
                 {lastTranslation && (
                   <div style={{
                     padding: '8px 10px',
                     background: 'var(--bg-color)',
                     border: '1px solid var(--card-border)',
                     borderTop: 'none',
                     borderRadius: '0 0 8px 8px',
                     minHeight: '68px',
                     display: 'flex',
                     alignItems: 'center',
                     width: '100%'
                   }}>
                     {audioUrl && (
                       <audio
                         ref={hiddenAudioRef}
                         src={audioUrl}
                         style={{ display: 'none' }}
                         onPlay={() => setIsAudioPlaying(true)}
                         onPause={() => setIsAudioPlaying(false)}
                         onEnded={() => setIsAudioPlaying(false)}
                         onLoadedMetadata={(e) => setAudioDuration(e.currentTarget.duration)}
                       />
                     )}
                     <div style={{ display: 'flex', alignItems: 'center', gap: '8px', width: '100%' }}>
                        <button
                          onClick={handlePlayPause}
                          disabled={!audioUrl}
                          style={{
                            width: '28px',
                            height: '28px',
                            background: audioUrl ? '#2ECA7F' : '#444',
                            color: '#000',
                            border: 'none',
                            borderRadius: '50%',
                            padding: '0',
                            cursor: audioUrl ? 'pointer' : 'not-allowed',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            opacity: audioUrl ? 1 : 0.5,
                            flexShrink: 0
                          }}
                        >
                          {audioUrl && (isAudioPlaying ? (
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                              <rect x="6" y="4" width="4" height="16"/>
                              <rect x="14" y="4" width="4" height="16"/>
                            </svg>
                          ) : (
                            <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                              <path d="M8 5v14l11-7z"/>
                            </svg>
                          ))}
                        </button>
                        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '4px', justifyContent: 'center' }}>
                          <div
                            onClick={handleSeek}
                            style={{
                              width: '100%',
                              height: '6px',
                              background: 'rgba(255,255,255,0.1)',
                              borderRadius: '3px',
                              cursor: audioUrl ? 'pointer' : 'default',
                              position: 'relative',
                              overflow: 'hidden'
                            }}
                          >
                            <div style={{
                              width: audioDuration > 0 ? `${(audioCurrentTime / audioDuration) * 100}%` : '0%',
                              height: '100%',
                              background: '#2ECA7F',
                              borderRadius: '3px',
                              transition: isAudioPlaying ? 'none' : 'width 0.1s linear'
                            }} />
                          </div>
                          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.65rem', color: 'var(--text-secondary)' }}>
                            <span>{audioCurrentTime.toFixed(1)}s</span>
                            <span>{audioDuration.toFixed(1)}s</span>
                          </div>
                        </div>
                      </div>
                   </div>
                 )}
               </div>

           </div>

         </div>
       </div>

       {/* Settings Modal */}
       {isSettingsOpen && (
         <div
           style={{
             position: 'fixed',
             top: 0,
             left: 0,
             right: 0,
             bottom: 0,
             background: 'rgba(0,0,0,0.6)',
             backdropFilter: 'blur(4px)',
             display: 'flex',
             alignItems: 'center',
             justifyContent: 'center',
             zIndex: 100,
           }}
           onClick={() => setIsSettingsOpen(false)}
         >
           <div
             style={{
               background: 'var(--card-bg)',
               border: '1px solid var(--card-border)',
               borderRadius: '12px',
               padding: '24px',
               width: '100%',
               maxWidth: '400px',
               margin: '20px',
             }}
             onClick={e => e.stopPropagation()}
           >
             <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
               <h3 style={{ margin: 0, fontSize: '1.1rem', color: 'var(--text-primary)' }}>Settings</h3>
               <button
                 onClick={() => setIsSettingsOpen(false)}
                 style={{ background: 'transparent', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', fontSize: '1.2rem' }}
               >
                 ×
               </button>
             </div>

             <div style={{ display: 'flex', flexDirection: 'column', gap: '20px' }}>
               <div>
                 <label style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '8px' }}>
                   <span>Voice Threshold</span>
                   <span style={{ color: '#2ECA7F' }}>{liveThresholdDb.toFixed(0)} dB</span>
                 </label>
                 <input
                   type="range"
                   min="-60"
                   max="0"
                   step="1"
                   value={liveThresholdDb}
                   onChange={e => setLiveThresholdDb(Number(e.target.value))}
                   style={{ width: '100%', accentColor: '#2ECA7F' }}
                 />
                 <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', marginTop: '4px' }}>
                   dB relative to full scale. Lower (more negative) = more sensitive.
                 </div>
               </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
                  <button
                    className="apply-btn"
                    onClick={calibrateThreshold}
                    disabled={calibrationStatus.status === 'started'}
                    style={{
                      opacity: calibrationStatus.status === 'started' ? 0.7 : 1,
                      cursor: calibrationStatus.status === 'started' ? 'wait' : 'pointer',
                    }}
                  >
                    {calibrationStatus.status === 'started'
                      ? `Calibrating... ${calibrateCountdown}s`
                      : calibrationStatus.status === 'complete'
                        ? 'Calibrated!'
                        : 'Auto Calibrate Threshold'}
                  </button>
                  <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>
                    Stay silent for 5 seconds. The system will measure ambient noise and set the threshold automatically.
                  </div>
                  {calibrationStatus.status === 'error' && (
                    <div style={{ fontSize: '0.75rem', color: '#ff453a' }}>
                      Error: {calibrationStatus.error}
                    </div>
                  )}
                </div>

                <div>
                  <label style={{ display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '8px' }}>
                    <span>Silence Duration</span>
                   <span style={{ color: '#2ECA7F' }}>{liveSilenceDuration.toFixed(1)}s</span>
                 </label>
                 <input
                   type="range"
                   min="0.5"
                   max="5.0"
                   step="0.1"
                   value={liveSilenceDuration}
                   onChange={e => setLiveSilenceDuration(Number(e.target.value))}
                   style={{ width: '100%', accentColor: '#2ECA7F' }}
                 />
                 <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', marginTop: '4px' }}>
                   Seconds of silence before processing the speech segment.
                 </div>
               </div>

               <button
                 className="apply-btn"
                 onClick={() => {
                   updateConfig({
                     live_threshold: dbToRms(liveThresholdDb),
                     live_silence_duration: liveSilenceDuration,
                   });
                   setIsSettingsOpen(false);
                 }}
               >
                 Save Settings
               </button>
             </div>
           </div>
         </div>
       )}
     </>
   );
 }

export default App;