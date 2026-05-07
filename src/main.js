const { invoke } = window.__TAURI__.core;

let isRecording = false;
let currentStatus = 'idle';

async function updateStatus() {
    try {
        const status = await invoke('get_status');
        updateUI(status);
    } catch (error) {
        console.error('Error getting status:', error);
    }
}

function updateUI(status) {
    const statusDot = document.getElementById('statusDot');
    const statusText = document.getElementById('statusText');
    const captureBtn = document.getElementById('captureBtn');
    const btnText = captureBtn.querySelector('.btn-text');
    const btnIcon = captureBtn.querySelector('.btn-icon');

    currentStatus = status.status;

    statusDot.className = 'status-dot';

    switch (status.status) {
        case 'idle':
            statusText.textContent = 'Aguardando';
            break;
        case 'loading':
            statusText.textContent = 'Carregando Modelos';
            statusDot.classList.add('processing');
            break;
        case 'ready':
            statusText.textContent = 'Sistema Online';
            statusDot.classList.add('ready');
            break;
        case 'recording':
            statusText.textContent = 'Capturando';
            statusDot.classList.add('recording');
            captureBtn.classList.add('recording');
            btnText.textContent = 'Parar';
            btnIcon.textContent = '⏹';
            break;
        case 'processing':
            statusText.textContent = 'Processando';
            statusDot.classList.add('processing');
            break;
        case 'error':
            statusText.textContent = 'Erro';
            statusDot.classList.add('error');
            break;
        default:
            statusText.textContent = status.status;
    }

    if (status.status !== 'recording') {
        captureBtn.classList.remove('recording');
        btnText.textContent = 'Iniciar';
        btnIcon.textContent = '▶';
    }

    updateResults(status);
    updateHardwareUI(status);

    localStorage.setItem('voicy_last_status', JSON.stringify(status));
}

function updateResults(status) {
    if (status.last_transcription) {
        const transcriptionEl = document.getElementById('transcriptionResult');
        if (transcriptionEl.innerHTML !== status.last_transcription) {
            animateResultUpdate(transcriptionEl, status.last_transcription);
        }
    }

    if (status.last_translation) {
        const translationEl = document.getElementById('translationResult');
        if (translationEl.innerHTML !== status.last_translation) {
            animateResultUpdate(translationEl, status.last_translation);
        }
    }
}

function animateResultUpdate(element, content) {
    element.style.transition = 'all 0.3s ease';
    element.style.opacity = '0';
    element.style.transform = 'translateX(-20px)';
    
    setTimeout(() => {
        element.innerHTML = content;
        element.style.opacity = '1';
        element.style.transform = 'translateX(0)';
    }, 150);
}

function updateHardwareUI(status) {
    const gpuStatus = document.getElementById('gpuStatus');
    if (status.has_cuda) {
        gpuStatus.innerHTML = `🟢 GPU: ${status.gpu_name || 'NVIDIA'} CUDA Ativo`;
        gpuStatus.className = 'gpu-status cuda';
    } else {
        gpuStatus.innerHTML = `🟡 Modo CPU Sem GPU`;
        gpuStatus.className = 'gpu-status cpu';
    }
}

async function toggleCapture() {
    try {
        const captureBtn = document.getElementById('captureBtn');
        
        captureBtn.style.transform = 'scale(0.95)';
        setTimeout(() => captureBtn.style.transform = '', 150);
        
        if (isRecording) {
            await invoke('stop_capture');
            isRecording = false;
        } else {
            await invoke('start_capture');
            isRecording = true;
        }
        await updateStatus();
    } catch (error) {
        console.error('Error toggling capture:', error);
        alert('Erro: ' + error);
    }
}

async function setLanguage(type, language) {
    try {
        const command = type === 'source' ? 'set_source_language' : 'set_target_language';
        await invoke(command, { language });
        
        const select = type === 'source' ? document.getElementById('sourceLanguage') : document.getElementById('targetLanguage');
        select.style.transition = 'all 0.2s ease';
        select.style.borderColor = 'var(--accent-magenta)';
        select.style.boxShadow = '0 0 20px rgba(255, 0, 255, 0.4)';
        
        setTimeout(() => {
            select.style.borderColor = '';
            select.style.boxShadow = '';
        }, 300);
    } catch (error) {
        console.error('Error setting language:', error);
    }
}

async function setMode(mode) {
    try {
        await invoke('set_mode', { mode });

        document.querySelectorAll('.mode-btn').forEach(btn => {
            btn.classList.remove('active');
        });
        
        const activeBtn = document.querySelector(`[data-mode="${mode}"]`);
        activeBtn.classList.add('active');
    } catch (error) {
        console.error('Error setting mode:', error);
    }
}

function copyToClipboard(elementId) {
    const element = document.getElementById(elementId);
    const text = element.textContent || element.innerText;
    const copyBtn = element.parentElement.querySelector('.copy-btn');
    
    navigator.clipboard.writeText(text).then(() => {
        copyBtn.classList.add('copied');
        copyBtn.textContent = '✓';
        
        setTimeout(() => {
            copyBtn.classList.remove('copied');
            copyBtn.textContent = '📋';
        }, 1500);
        
        playCopySound();
    }).catch(err => {
        console.error('Failed to copy:', err);
    });
}

function playCopySound() {
    const audioContext = new (window.AudioContext || window.webkitAudioContext)();
    const oscillator = audioContext.createOscillator();
    const gainNode = audioContext.createGain();
    
    oscillator.connect(gainNode);
    gainNode.connect(audioContext.destination);
    
    oscillator.frequency.setValueAtTime(800, audioContext.currentTime);
    oscillator.frequency.exponentialRampToValueAtTime(1200, audioContext.currentTime + 0.1);
    
    gainNode.gain.setValueAtTime(0.1, audioContext.currentTime);
    gainNode.gain.exponentialRampToValueAtTime(0.01, audioContext.currentTime + 0.2);
    
    oscillator.start(audioContext.currentTime);
    oscillator.stop(audioContext.currentTime + 0.2);
}

function createParticleEffect(x, y) {
    for (let i = 0; i < 8; i++) {
        const particle = document.createElement('div');
        particle.style.cssText = `
            position: fixed;
            left: ${x}px;
            top: ${y}px;
            width: 6px;
            height: 6px;
            background: var(--accent-cyan);
            border-radius: 50%;
            pointer-events: none;
            z-index: 9999;
            box-shadow: 0 0 10px var(--accent-cyan);
        `;
        
        const angle = (i / 8) * Math.PI * 2;
        const velocity = 100 + Math.random() * 50;
        const dx = Math.cos(angle) * velocity;
        const dy = Math.sin(angle) * velocity;
        
        document.body.appendChild(particle);
        
        let opacity = 1;
        let posX = x;
        let posY = y;
        
        const animateParticle = () => {
            posX += dx * 0.02;
            posY += dy * 0.02;
            opacity -= 0.03;
            
            particle.style.left = posX + 'px';
            particle.style.top = posY + 'px';
            particle.style.opacity = opacity;
            
            if (opacity > 0) {
                requestAnimationFrame(animateParticle);
            } else {
                particle.remove();
            }
        };
        
        requestAnimationFrame(animateParticle);
    }
}

function addHoverEffects() {
    const captureBtn = document.getElementById('captureBtn');
    
    captureBtn.addEventListener('click', (e) => {
        createParticleEffect(
            captureBtn.getBoundingClientRect().left + captureBtn.offsetWidth / 2,
            captureBtn.getBoundingClientRect().top + captureBtn.offsetHeight / 2
        );
    });

    document.querySelectorAll('.mode-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            createParticleEffect(e.clientX, e.clientY);
        });
    });

    document.querySelectorAll('.copy-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            for (let i = 0; i < 12; i++) {
                setTimeout(() => {
                    createParticleEffect(
                        e.clientX + (Math.random() - 0.5) * 30,
                        e.clientY + (Math.random() - 0.5) * 30
                    );
                }, i * 30);
            }
        });
    });
}

function initKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
        if (e.ctrlKey && e.shiftKey && e.key === 'V') {
            e.preventDefault();
            const captureBtn = document.getElementById('captureBtn');
            const rect = captureBtn.getBoundingClientRect();
            createParticleEffect(rect.left + rect.width/2, rect.top + rect.height/2);
            toggleCapture();
        }
        
        if (e.ctrlKey && e.shiftKey && e.key === 'M') {
            e.preventDefault();
            const modes = ['auto', 'manual', 'live', 'transcription'];
            const currentModeBtn = document.querySelector('.mode-btn.active');
            const currentIndex = modes.indexOf(currentModeBtn.dataset.mode);
            const nextIndex = (currentIndex + 1) % modes.length;
            setMode(modes[nextIndex]);
        }
    });
}

async function checkModelsStatus() {
    try {
        const modelsInfo = await invoke('get_models_info');
        
        modelsInfo.forEach(model => {
            let id = '';
            if (model.folder.includes('whisper')) id = 'whisperStatus';
            else if (model.folder.includes('translate')) id = 'translateStatus';
            else if (model.folder.includes('omnivoice')) id = 'ttsStatus';
            
            const statusEl = document.getElementById(id);
            
            if (statusEl) {
                if (model.status === 'Downloaded') {
                    statusEl.textContent = '✓ Instalado';
                    statusEl.className = 'model-status downloaded';
                } else if (model.status === 'Downloading') {
                    statusEl.textContent = '⏳ Baixando';
                    statusEl.className = 'model-status downloading';
                } else {
                    statusEl.textContent = '⏳ Não instalado';
                    statusEl.className = 'model-status not-downloaded';
                }
            }
        });
    } catch (error) {
        console.error('Error checking models:', error);
    }
}

async function downloadModels() {
    const downloadBtn = document.getElementById('downloadModelsBtn');
    const progressSection = document.getElementById('downloadProgress');
    const progressFill = document.getElementById('progressFill');
    const progressText = document.getElementById('progressText');
    
    downloadBtn.disabled = true;
    downloadBtn.textContent = 'Baixando...';
    progressSection.style.display = 'block';
    
    try {
        const modelsInfo = await invoke('get_models_info');
        const totalModels = modelsInfo.length;
        let downloadedCount = 0;
        
        for (const model of modelsInfo) {
            if (model.status === 'Downloaded') {
                downloadedCount++;
                continue;
            }
            
            progressText.textContent = `Baixando ${model.name}...`;
            
            let id = '';
            if (model.folder.includes('whisper')) id = 'whisperStatus';
            else if (model.folder.includes('translate')) id = 'translateStatus';
            else if (model.folder.includes('omnivoice')) id = 'ttsStatus';
            
            const modelStatusEl = document.getElementById(id);
            if (modelStatusEl) {
                modelStatusEl.textContent = '⏳ Baixando';
                modelStatusEl.className = 'model-status downloading';
            }
            
            const progress = ((downloadedCount / totalModels) * 100);
            progressFill.style.width = `${progress}%`;
            
            try {
                await invoke('download_model', { folder: model.folder });
            } catch (e) {
                console.error("Failed to download model:", e);
            }
            await new Promise(resolve => setTimeout(resolve, 1500));
            
            downloadedCount++;
        }
        
        progressFill.style.width = '100%';
        progressText.textContent = 'Download completo!';
        
        setTimeout(() => {
            progressSection.style.display = 'none';
            progressFill.style.width = '0%';
        }, 2000);
        
        await checkModelsStatus();
        
        try {
            await invoke('reload_models');
        } catch (e) {
            console.error("Failed to reload models:", e);
        }
        
        downloadBtn.textContent = '✓ Baixado';
        
    } catch (error) {
        console.error('Error downloading models:', error);
        progressText.textContent = 'Erro no download: ' + error;
        downloadBtn.disabled = false;
        downloadBtn.textContent = '⬇ Tentar Novamente';
    }
}

async function loadAudioDevices() {
    try {
        const devices = await invoke('list_audio_devices');
        const inputSelect = document.getElementById('inputDevice');
        const outputSelect = document.getElementById('outputDevice');
        
        devices.inputs.forEach(device => {
            const option = document.createElement('option');
            option.value = device;
            option.textContent = '◈ ' + device;
            inputSelect.appendChild(option);
        });
        
        devices.outputs.forEach(device => {
            const option = document.createElement('option');
            option.value = device;
            option.textContent = '◈ ' + device;
            outputSelect.appendChild(option);
        });
    } catch (e) {
        console.error("Falha ao carregar dispositivos de audio:", e);
    }
}

async function init() {
    console.log('Initializing Voicy...');

    await loadAudioDevices();

    const captureBtn = document.getElementById('captureBtn');
    captureBtn.addEventListener('click', toggleCapture);

    const sourceSelect = document.getElementById('sourceLanguage');
    sourceSelect.addEventListener('change', (e) => {
        setLanguage('source', e.target.value);
    });

    const targetSelect = document.getElementById('targetLanguage');
    targetSelect.addEventListener('change', (e) => {
        setLanguage('target', e.target.value);
    });

    document.querySelectorAll('.mode-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            setMode(e.currentTarget.dataset.mode);
        });
    });

    document.getElementById('copyTranscription').addEventListener('click', () => {
        copyToClipboard('transcriptionResult');
    });

    document.getElementById('copyTranslation').addEventListener('click', () => {
        copyToClipboard('translationResult');
    });

    addHoverEffects();
    initKeyboardShortcuts();

    document.getElementById('downloadModelsBtn').addEventListener('click', () => {
        createParticleEffect(
            document.getElementById('downloadModelsBtn').getBoundingClientRect().left + 60,
            document.getElementById('downloadModelsBtn').getBoundingClientRect().top + 20
        );
        downloadModels();
    });

    try {
        const hardwareInfo = await invoke('get_hardware_info');
        const gpuStatus = document.getElementById('gpuStatus');
        
        if (hardwareInfo.has_cuda) {
            gpuStatus.innerHTML = `🟢 GPU: ${hardwareInfo.gpu_name || 'NVIDIA'} CUDA Ativo`;
            gpuStatus.className = 'gpu-status cuda';
        } else {
            gpuStatus.innerHTML = `🟡 Modo CPU Sem GPU`;
            gpuStatus.className = 'gpu-status cpu';
        }
    } catch (error) {
        console.error('Error getting hardware info:', error);
    }

    try {
        await checkModelsStatus();
    } catch (error) {
        console.error('Error checking models:', error);
    }

    setInterval(updateStatus, 1000);

    await updateStatus();
    
    console.log('Voicy initialized');
}

document.addEventListener('DOMContentLoaded', init);