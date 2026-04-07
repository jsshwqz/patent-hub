@echo off
chcp 65001 >nul 2>nul
cd /d "%~dp0"

REM Kill existing instance if running
taskkill /F /IM innoforge.exe >nul 2>nul
timeout /t 1 /nobreak >nul

echo [InnoForge] Starting...
echo [InnoForge] Building...
cargo build --release --bin innoforge
if errorlevel 1 (
    echo [InnoForge] Build failed!
    pause
    exit /b 1
)
echo [InnoForge] Launching...
.\target\release\innoforge.exe
pause
