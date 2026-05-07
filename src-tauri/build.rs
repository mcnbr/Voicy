fn main() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    if target_arch == "x86_64" {
        if cfg!(feature = "cuda") {
            println!("cargo:rustc-cfg=cuda_enabled");
            println!("cargo:warning=Building with CUDA support enabled");
        }

        if detect_nvidia_cuda() {
            println!("cargo:rustc-cfg=cuda_available");
            println!("cargo:warning=NVIDIA CUDA detected, enabling GPU acceleration");
        } else if cfg!(feature = "cuda") {
            println!("cargo:warning=CUDA feature enabled but CUDA not detected, falling back to CPU");
        }
    }

    tauri_build::build();
}

fn detect_nvidia_cuda() -> bool {
    #[cfg(windows)]
    {
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
    {
        false
    }
}