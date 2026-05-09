$env:CUDA_NVCC_FLAGS = "--allow-unsupported-compiler"

$originalPath = $env:PATH
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$env:PATH = "$scriptDir;$originalPath"

Write-Host "CUDA_NVCC_FLAGS = $($env:CUDA_NVCC_FLAGS)" -ForegroundColor Cyan
Write-Host "NVCC wrapper added to PATH" -ForegroundColor Cyan

Set-Location -LiteralPath "$scriptDir\src-tauri"
cargo build