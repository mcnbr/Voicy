@echo off
echo ==========================================================
echo Inicializando ambiente do compilador C++ (Visual Studio)
echo ==========================================================
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

echo.
echo ==========================================================
echo Compilando Voicy com suporte a GPU NVIDIA (CUDA)...
echo ==========================================================
npm run tauri-dev
