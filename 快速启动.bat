@echo off
chcp 65001 >nul

cd /d "%~dp0"

REM 检查是否已编译
if not exist "target\release\patent-hub.exe" (
    echo 首次运行，正在编译...
    call build.bat
)

REM 启动服务器
call start.bat