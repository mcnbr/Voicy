@echo off
setlocal EnableDelayedExpansion

echo ========================================
echo   Voicy - Setup e Inicializacao
echo ========================================
echo.

cd /d "%~dp0"

if exist "node_modules" (
    echo [SKIP] node_modules ja existe
) else (
    echo [OK] Instalando dependências npm...
    call npm install
    if errorlevel 1 (
        echo [ERRO] Falha ao instalar npm
        pause
        exit /b 1
    )
)

if exist "src-tauri\target\debug\voicy_lib.dll" (
    echo [SKIP] Backend ja compilado
) else (
    echo [OK] Compilando backend Rust...
    powershell -ExecutionPolicy Bypass -File "%~dp0build.ps1"
    if errorlevel 1 (
        echo [ERRO] Falha ao compilar Rust
        pause
        exit /b 1
    )
)

echo.
echo ========================================
echo   Iniciando Voicy...
echo ========================================
echo.

npm run tauri-dev