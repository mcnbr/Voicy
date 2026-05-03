use log::info;
use std::process::Command;

use crate::app_state::HardwareInfo;

pub fn detect_hardware() -> HardwareInfo {
    let mut info = HardwareInfo::default();

    info.has_cuda = check_cuda();

    if info.has_cuda {
        info!(" CUDA GPU detected");
    } else {
        info!(" Running on CPU only");
    }

    info
}

fn check_cuda() -> bool {
    #[cfg(windows)]
    {
        let output = Command::new("nvidia-smi")
            .arg("--query-gpu=name")
            .arg("--format=csv,noheader")
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let gpu_name = String::from_utf8_lossy(&output.stdout);
                if !gpu_name.trim().is_empty() {
                    return true;
                }
            }
        }
        false
    }

    #[cfg(not(windows))]
    {
        false
    }
}