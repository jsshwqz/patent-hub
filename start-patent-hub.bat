@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo Starting Patent Hub...
echo Building project...
cargo build --release --bin patent-hub
if errorlevel 1 (
    echo Build failed!
    pause
    exit /b 1
)
echo Launching application...
timeout /t 2 /nobreak
start http://127.0.0.1:3000/search
.\target\release\patent-hub.exe
pause
