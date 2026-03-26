@echo off
chcp 65001 >nul
cd /d "%~dp0"
echo 正在启动 Patent Hub...
echo 正在构建项目...
cargo build --release --bin patent-hub
if errorlevel 1 (
    echo 构建失败！
    pause
    exit /b 1
)
echo 正在启动应用...
timeout /t 2 /nobreak
start http://127.0.0.1:3000/search
.\target\release\patent-hub.exe
pause
