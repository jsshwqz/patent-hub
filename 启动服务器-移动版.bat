@echo off
chcp 65001 >nul
title Patent Hub 服务器（支持移动设备访问）

echo ========================================
echo   Patent Hub 专利检索系统
echo   支持电脑、手机、平板访问
echo ========================================
echo.

cd /d "%~dp0"

if not exist "target\release\patent-hub.exe" (
    echo [错误] 未找到可执行文件
    echo 请先运行: cargo build --release
    pause
    exit /b 1
)

echo [启动] 正在启动服务器...
echo.

start "" "target\release\patent-hub.exe"

timeout /t 3 /nobreak >nul

echo [提示] 服务器已启动！
echo.
echo 访问方式：
echo   电脑浏览器: http://127.0.0.1:3000
echo   手机/平板:   查看终端显示的 IP 地址
echo.
echo 如何用手机访问：
echo   1. 确保手机和电脑连接同一个 WiFi
echo   2. 在手机浏览器输入上面显示的 IP 地址
echo   3. 详见 docs\MOBILE_ACCESS.md
echo.
echo [提示] 按 Ctrl+C 停止服务器
echo ========================================

start http://127.0.0.1:3000

pause
