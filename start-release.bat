@echo off
chcp 65001 >nul
title Patent Hub 服务器
echo ========================================
echo    Patent Hub 专利检索系统
echo ========================================
echo.
echo 正在启动服务器...
echo.

cd /d "%~dp0"
if not exist "patent-hub.exe" (
    echo 错误: 找不到 patent-hub.exe
    echo 请确保此文件与 patent-hub.exe 在同一目录
    pause
    exit /b 1
)

echo 服务器地址: http://127.0.0.1:3000
echo.
echo 按 Ctrl+C 停止服务器
echo ========================================
echo.

patent-hub.exe

pause
