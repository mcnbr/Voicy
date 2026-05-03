# Voicy — Product Requirements Document (PRD)

> **Versão:** 1.0.0  
> **Data:** 2026-05-01  
> **Autor:** mcnbr  
> **Licença:** MIT License (com restrição de uso comercial)

---

## 1. Visão Geral do Produto

**Voicy** é uma aplicação desktop nativa que realiza, em tempo real, o pipeline completo de:

1. **Transcrição** de áudio (Speech-to-Text) usando **Whisper Large V3 Turbo**
2. **Tradução** do texto transcrito usando **TranslateGemma 4B**
3. **Síntese vocal** (Text-to-Speech) do texto traduzido usando **OmniVoice**

A aplicação é construída inteiramente em **Rust** com **Tauri** para a interface gráfica, executando todos os modelos de IA localmente na máquina do usuário — sem dependência de APIs externas.

---

## 2. Objetivos

| # | Objetivo | Métrica de Sucesso |
|---|----------|--------------------|
| 1 | Latência mínima no pipeline completo | < 3s do fim da fala até início do áudio traduzido (GPU) |
| 2 | Execução 100% offline | Nenhuma requisição de rede após download dos modelos |
| 3 | Aceleração por GPU quando disponível | Detecção automática de CUDA; fallback para CPU com aviso |
| 4 | Interface intuitiva e responsiva | Feedback visual em tempo real de cada etapa do pipeline |
| 5 | Consumo de recursos otimizado | Todos os 3 modelos carregados simultaneamente em VRAM/RAM |

---

## 3. Público-Alvo

- **Gamers** que precisam de tradução em tempo real de voice chat
- **Criadores de conteúdo** que produzem conteúdo multilíngue
- **Profissionais** em reuniões internacionais
- **Estudantes** consumindo conteúdo em idiomas estrangeiros
- **Usuários com deficiência auditiva** que precisam de transcrição em tempo real

---

## 4. Stack Tecnológica

### 4.1 Core

| Componente | Tecnologia | Justificativa |
|------------|------------|---------------|
| Linguagem | **Rust** | Performance nativa, segurança de memória, zero-cost abstractions |
| Framework UI | **Tauri 2** | Bundle leve, WebView nativo, API Rust rica |
| Frontend | **HTML/CSS/JS** (vanilla ou React) | Renderização no WebView do Tauri |

### 4.2 Modelos de IA

| Função | Modelo | Framework de Inferência | Formato |
|--------|--------|------------------------|---------|
| STT (Speech-to-Text) | **Faster Whisper Large V3** | `candle` (HuggingFace) | GGUF (quantizado q4_k) |
| Tradução | **TranslateGemma 4B** | `llama-cpp-2` (bindings Rust) | GGUF (quantizado) |
| TTS (Text-to-Speech) | **OmniVoice** | `candle` / `omnivoice-rs` | Nativo |

### 4.3 Aceleração de Hardware

| Backend | Biblioteca | Condição |
|---------|-----------|----------|
| **CUDA** (NVIDIA) | `candle-cuda`, `llama-cpp` com CUDA | GPU NVIDIA com CUDA Toolkit detectado |
| **CPU** (Fallback) | `candle-core` CPU | Sem GPU CUDA disponível — exibe aviso ao usuário |

---

## 5. Requisitos Funcionais

### RF-01: Captura de Áudio
- Capturar áudio do microfone do sistema em tempo real
- Suportar seleção de dispositivo de entrada
- Buffer de áudio com janela deslizante configurável
- Indicador visual de nível de áudio (VU meter)
- Detecção de silêncio (VAD) para segmentar frases automaticamente

### RF-02: Transcrição (Whisper Large V3 Turbo)
- Transcrever áudio capturado para texto
- Suportar múltiplos idiomas de entrada (auto-detect ou seleção manual)
- Exibir texto transcrito em tempo real na interface
- Modelo carregado na inicialização e mantido em memória

### RF-03: Tradução (TranslateGemma 4B)
- Traduzir texto transcrito para o idioma de destino selecionado
- Suportar pares de idiomas configuráveis
- Exibir texto traduzido na interface
- Opção de bypass (pular tradução quando idioma de origem = destino)
- Modelo carregado na inicialização e mantido em memória

### RF-04: Síntese Vocal (OmniVoice)
- Gerar áudio a partir do texto traduzido
- Reproduzir áudio gerado automaticamente
- Suportar seleção de voz/estilo
- Controle de volume e velocidade
- Modelo carregado na inicialização e mantido em memória

### RF-05: Carregamento Simultâneo de Modelos
- Todos os 3 modelos devem ser carregados ao iniciar a aplicação
- Barra de progresso durante o carregamento
- Relatório de VRAM/RAM utilizada por cada modelo
- Se GPU CUDA disponível: carregar todos na VRAM
- Se não: carregar na RAM com aviso proeminente ao usuário

### RF-06: Detecção de Hardware
- Detectar automaticamente se GPU NVIDIA com CUDA está disponível
- Se CUDA detectado: usar GPU para todos os modelos
- Se CUDA não disponível: fallback para CPU com aviso claro:
  > ⚠️ "Nenhuma GPU CUDA detectada. O Voicy está rodando no processador (CPU). O desempenho será significativamente reduzido e o programa pode não funcionar de maneira adequada."

### RF-07: Interface do Usuário

- **AudioWave Visualizer:** Indicador visual em tempo real que mostra se a voz do usuário está sendo captada
  - Visualização de forma de onda em tempo real
  - Cores diferentes para: captando (ativo), silêncio, processando
- **Seletor de Input de Áudio:** Dropdown para selecionar o dispositivo de entrada (microfone, virtual cable, etc.)
- **Seletor de Idioma de Origem:** Idioma que o usuário vai falar (com opção "Auto-detect")
- **Seletor de Idioma de Destino:** Idioma de saída tradução
- **Seletor de Output de Áudio:** Dispositivo de saída onde o áudio será reproduzido
- **Cards de Status dos Modelos:** Três cards exibindo:
  - Nome do modelo
  - Tempo de processamento (em milissegundos)
  - Output desse estágio (texto para Whisper/TranslateGemma, player de áudio para OmniVoice)
- **Player de Áudio (OmniVoice):** Com controles estilo Spotify:
  - Play/Pause
  - Barra de progresso
  - Tempo atual / Tempo total
  - Volume controlável
- **Indicador de Status do Pipeline:** Ocioso → Transcrevendo → Traduzindo → Sintetizando → Reproduzindo
- **Indicador de Hardware:** GPU (verde) / CPU (amarelo) com VRAM/RAM utilizada

### RF-08: Modos de Operação

| Modo | Descrição | Comportamento |
|------|-----------|--------------|
| **Automático** | O usuário clica para gravar e para a gravação | O Voicy processa o áudio e toca automaticamente no output pré-definido |
| **Manual** | O usuário clica para gravar e para a gravação | O Voicy processa e monitora o input de áudio (Virtual Cable/Microfone). Quando qualquer aplicativo "chamar" o input de áudio (ex: WhatsApp, Discord), o Voicy toca o último áudio processado uma vez. Se chamado novamente, toca novamente |
| **Live** | O usuário inicia a captura contínua | O sistema traduz continuamente o que for dito. O usuário define um "lote" de áudio através de uma pausa na fala. O tempo de pausa é configurável pelo usuário |
| **Transcrição** | Modo de texto apenas | O texto traduzido pelo TranslateGemma vai para a área de transferência (Ctrl+V). O OmniVoice é desabilitado |

### RF-09: Configurações Persistentes
- Salvar preferências do usuário entre sessões
- Idiomas selecionados, dispositivo de áudio, modo de operação
- Caminho dos modelos, configurações de voz TTS

---

## 6. Requisitos Não-Funcionais

### RNF-01: Performance
- Carregamento inicial dos 3 modelos: < 30s (GPU) / < 60s (CPU)
- Latência STT: < 1s após fim do segmento de fala
- Latência tradução: < 500ms por sentença
- Latência TTS: < 1s para início da reprodução
- Pipeline completo end-to-end: < 3s (GPU)

### RNF-02: Recursos de Sistema
- RAM mínima: 16 GB
- VRAM recomendada: 8 GB+ (para os 3 modelos simultâneos)
- Disco: ~10 GB para modelos + aplicação
- CPU mínimo: 8 cores (para modo CPU)

### RNF-03: Compatibilidade
- Windows 10/11 (64-bit) — plataforma primária
- Linux (futuro)
- macOS (futuro)

### RNF-04: Segurança
- Toda inferência local — nenhum dado enviado para servidores externos
- Nenhuma telemetria ou coleta de dados do usuário

### RNF-05: Distribuição
- Distribuído como executável portátil (ZIP)
- Modelos baixados separadamente ou via download manager integrado
- Sem necessidade de instalação de Python, Node.js ou outros runtimes

---

## 7. Arquitetura de Alto Nível

```
┌─────────────────────────────────────────────────────────┐
│                      TAURI SHELL                        │
│  ┌───────────────────────────────────────────────────┐  │
│  │              Frontend (WebView)                   │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │  │
│  │  │ Painel   │ │ Painel   │ │ Controles &      │  │  │
│  │  │ STT      │ │ Tradução │ │ Configurações    │  │  │
│  │  └──────────┘ └──────────┘ └──────────────────┘  │  │
│  └──────────────────┬────────────────────────────────┘  │
│                     │ Tauri IPC (Commands)               │
│  ┌──────────────────┴────────────────────────────────┐  │
│  │              Backend Rust                         │  │
│  │                                                   │  │
│  │  ┌─────────┐  ┌──────────────┐  ┌─────────────┐  │  │
│  │  │ Audio   │  │ Pipeline     │  │ Config      │  │  │
│  │  │ Capture │──│ Manager      │──│ Manager     │  │  │
│  │  │ (cpal)  │  │              │  │             │  │  │
│  │  └─────────┘  └──────┬───────┘  └─────────────┘  │  │
│  │                      │                            │  │
│  │         ┌────────────┼────────────┐               │  │
│  │         ▼            ▼            ▼               │  │
│  │  ┌───────────┐ ┌──────────┐ ┌──────────┐         │  │
│  │  │ Whisper   │ │Translate │ │OmniVoice │         │  │
│  │  │ V3 Turbo  │ │ Gemma 4B │ │  TTS     │         │  │
│  │  │ (candle)  │ │(llama-cpp)│ │(candle)  │         │  │
│  │  └─────┬─────┘ └────┬─────┘ └────┬─────┘         │  │
│  │        │             │            │               │  │
│  │        └─────────────┴────────────┘               │  │
│  │                      │                            │  │
│  │              ┌───────┴───────┐                    │  │
│  │              │ CUDA / CPU    │                    │  │
│  │              │ Backend       │                    │  │
│  │              └───────────────┘                    │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

---

## 8. Estrutura do Projeto

```
voicy/
├── src-tauri/                  # Backend Rust (Tauri)
│   ├── Cargo.toml              # Dependências Rust
│   ├── build.rs                # Build script (CUDA detection)
│   ├── tauri.conf.json         # Configuração do Tauri
│   └── src/
│       ├── main.rs             # Entry point
│       ├── lib.rs              # Módulo raiz
│       ├── app_state.rs        # Estado global da aplicação
│       ├── commands/           # Tauri commands (IPC)
│       │   ├── mod.rs
│       │   ├── audio.rs        # Comandos de áudio
│       │   ├── pipeline.rs     # Comandos do pipeline
│       │   └── config.rs       # Comandos de configuração
│       ├── audio/              # Captura e reprodução de áudio
│       │   ├── mod.rs
│       │   ├── capture.rs      # Captura do microfone
│       │   ├── playback.rs     # Reprodução de áudio
│       │   └── vad.rs          # Voice Activity Detection
│       ├── models/             # Gerenciamento de modelos de IA
│       │   ├── mod.rs
│       │   ├── whisper.rs      # Whisper Large V3 Turbo
│       │   ├── translate.rs    # TranslateGemma 4B
│       │   └── tts.rs          # OmniVoice
│       ├── pipeline/           # Orquestração do pipeline
│       │   ├── mod.rs
│       │   └── manager.rs      # Pipeline manager
│       ├── config/             # Configuração persistente
│       │   ├── mod.rs
│       │   └── settings.rs     # Struct de configurações
│       └── hardware/           # Detecção de hardware
│           ├── mod.rs
│           └── cuda.rs         # Detecção CUDA
├── src/                        # Frontend (WebView)
│   ├── index.html
│   ├── index.css
│   ├── main.js
│   └── components/
├── docs/                       # Documentação
│   └── PRD.md                  # Este documento
├── models/                     # Diretório para modelos (gitignored)
├── .gitignore
├── LICENSE
├── README.md
└── package.json
```

---

## 9. Dependências Rust Principais

| Crate | Uso |
|-------|-----|
| `tauri` | Framework de aplicação desktop |
| `candle-core` | Framework ML para Whisper e OmniVoice |
| `candle-transformers` | Modelos pré-treinados (Whisper) |
| `candle-nn` | Camadas de redes neurais |
| `candle-cuda` | Backend CUDA para candle |
| `llama-cpp-2` | Bindings Rust para llama.cpp (TranslateGemma) |
| `cpal` | Captura e reprodução de áudio multiplataforma |
| `hound` | Leitura/escrita de arquivos WAV |
| `symphonia` | Decodificação de áudio |
| `serde` / `serde_json` | Serialização de configurações |
| `tokio` | Runtime assíncrono |
| `anyhow` / `thiserror` | Tratamento de erros |

---

## 10. Fluxo do Pipeline

```
┌──────────┐    ┌──────────┐    ┌──────────────┐    ┌──────────┐    ┌──────────┐
│ Microfone│───▶│  VAD     │───▶│  Whisper     │───▶│Translate │───▶│OmniVoice │
│ (cpal)   │    │(Silêncio)│    │  V3 Turbo    │    │ Gemma 4B │    │  TTS     │
│          │    │          │    │              │    │          │    │          │
│  Áudio   │    │ Segmentos│    │  Texto PT    │    │ Texto EN │    │ Áudio EN │
│  Raw     │    │ de Fala  │    │  "Olá mundo" │    │"Hello    │    │ ▶ 🔊    │
│          │    │          │    │              │    │  world"  │    │          │
└──────────┘    └──────────┘    └──────────────┘    └──────────┘    └──────────┘
```

---

## 11. Milestones

### M1 — Fundação (Semana 1-2)
- [ ] Inicializar projeto Tauri + Rust
- [ ] Configurar build system com feature flags (cuda/cpu)
- [ ] Implementar detecção de hardware (CUDA)
- [ ] Implementar captura de áudio básica (cpal)
- [ ] Layout básico do frontend

### M2 — Whisper STT (Semana 3-4)
- [ ] Integrar candle com Whisper Large V3 Turbo
- [ ] Implementar carregamento do modelo GGUF
- [ ] Pipeline de pré-processamento de áudio (resampling, mel spectrogram)
- [ ] Transcrição funcional com exibição no frontend
- [ ] VAD para segmentação automática

### M3 — TranslateGemma (Semana 5-6)
- [ ] Integrar llama-cpp-2 com TranslateGemma 4B
- [ ] Carregamento do modelo GGUF com CUDA
- [ ] Pipeline de tradução com prompt engineering
- [ ] Integração no pipeline após STT

### M4 — OmniVoice TTS (Semana 7-8)
- [ ] Integrar OmniVoice via candle/omnivoice-rs
- [ ] Geração de áudio a partir de texto
- [ ] Reprodução de áudio gerado (cpal output)
- [ ] Pipeline completo end-to-end funcional

### M5 — Polish & Release (Semana 9-10)
- [ ] Interface refinada e responsiva
- [ ] Configurações persistentes
- [ ] Modos de operação (Auto, Push-to-Talk, Clipboard)
- [ ] Testes de performance e otimização
- [ ] Empacotamento e distribuição portátil
- [ ] Documentação final

---

## 12. Riscos e Mitigações

| Risco | Impacto | Probabilidade | Mitigação |
|-------|---------|---------------|-----------|
| VRAM insuficiente para 3 modelos simultâneos | Alto | Médio | Quantização agressiva; offload parcial para RAM |
| Latência alta no modo CPU | Alto | Alto | Aviso claro ao usuário; otimização com MKL/OpenBLAS |
| Incompatibilidade de modelos GGUF | Médio | Baixo | Pinning de versões; testes de compatibilidade |
| Build complexity com CUDA no Windows | Médio | Médio | Documentação detalhada; scripts de build automatizados |

---

## 13. Métricas de Sucesso

- **Pipeline completo GPU:** < 3 segundos end-to-end
- **Startup com 3 modelos:** < 30 segundos (GPU)
- **Uso de VRAM:** < 12 GB para os 3 modelos quantizados
- **Crash rate:** < 1% durante uso normal
- **Satisfação do usuário:** Interface clara com feedback visual em cada etapa

---

*Este documento será atualizado conforme o desenvolvimento avança.*
