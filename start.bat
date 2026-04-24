@echo off
chcp 65001 >nul 2>nul
cd /d "%~dp0"

REM Kill existing instance if running
taskkill /F /IM innoforge.exe >nul 2>nul
timeout /t 3 /nobreak >nul

echo [InnoForge] Building...
if exist Cargo.toml (
    cargo build --release --bin innoforge
    if errorlevel 1 (
        echo [InnoForge] Build failed!
        pause
        exit /b 1
    )
    set "APP=.\target\release\innoforge.exe"
) else (
    set "APP=.\innoforge.exe"
)

echo [InnoForge] Launching...
"%APP%"
pause
