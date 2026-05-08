use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub has_cuda: bool,
    pub gpu_name: Option<String>,
    pub vram_gb: Option<u32>,
    pub cpu_name: String,
    pub cpu_cores: u32,
    pub ram_gb: u32,
    pub active_device: String,  // "GPU: <name>" or "CPU: <name>"
}

impl HardwareInfo {
    pub fn detect() -> Self {
        let has_cuda = Self::detect_cuda();
        let (gpu_name, vram_gb) = if has_cuda {
            Self::get_gpu_info()
        } else {
            (None, None)
        };

        let cpu_cores = std::thread::available_parallelism()
            .map(|p| p.get() as u32)
            .unwrap_or(4);
        let ram_gb = Self::get_ram_gb();
        let cpu_name = Self::get_cpu_name();

        let active_device = if has_cuda {
            if let Some(ref name) = gpu_name {
                format!("GPU: {}", name)
            } else {
                "GPU: NVIDIA (unknown)".to_string()
            }
        } else {
            format!("CPU: {}", cpu_name)
        };

        Self {
            has_cuda,
            gpu_name,
            vram_gb,
            cpu_name,
            cpu_cores,
            ram_gb,
            active_device,
        }
    }

    #[cfg(windows)]
    fn detect_cuda() -> bool {
        use std::process::Command;
        let output = Command::new("nvidia-smi")
            .arg("-L")
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            return stdout.contains("NVIDIA");
        }
        false
    }

    #[cfg(not(windows))]
    fn detect_cuda() -> bool {
        false
    }

    #[cfg(windows)]
    fn get_gpu_info() -> (Option<String>, Option<u32>) {
        use std::process::Command;
        let output = Command::new("nvidia-smi")
            .arg("--query-gpu=name,memory.total")
            .arg("--format=csv,noheader")
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = stdout.trim().split(',').collect();
            if parts.len() >= 2 {
                let name = Some(parts[0].trim().to_string());
                let vram = parts[1]
                    .trim()
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<u32>().ok())
                    .map(|mb| mb / 1024);
                return (name, vram);
            }
        }
        (None, None)
    }

    #[cfg(not(windows))]
    fn get_gpu_info() -> (Option<String>, Option<u32>) {
        (None, None)
    }

    pub fn detect_cpu_name() -> String {
        Self::get_cpu_name()
    }

    fn get_cpu_name() -> String {
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("wmic")
                .args(["cpu", "get", "name", "/value"])
                .output();
            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.starts_with("Name=") {
                        let name = line.trim_start_matches("Name=").trim().to_string();
                        if !name.is_empty() {
                            return name;
                        }
                    }
                }
            }
            "Unknown CPU".to_string()
        }

        #[cfg(not(windows))]
        {
            "Unknown CPU".to_string()
        }
    }

    fn get_ram_gb() -> u32 {
        #[cfg(windows)]
        {
            use std::process::Command;
            let output = Command::new("wmic")
                .args(["OS", "get", "TotalVisibleMemorySize", "/Value"])
                .output();

            if let Ok(output) = output {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.starts_with("TotalVisibleMemorySize=") {
                        let kb: u64 = line
                            .split('=')
                            .nth(1)
                            .and_then(|s| s.trim().parse().ok())
                            .unwrap_or(0);
                        return (kb / 1024 / 1024) as u32;
                    }
                }
            }
            16
        }

        #[cfg(not(windows))]
        {
            16
        }
    }
}