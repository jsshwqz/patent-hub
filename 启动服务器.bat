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
start /min "" "target\release\patent-hub.exe"
timeout /t 3 >nul
echo ✓ 服务器已启动
echo.
echo 访问地址: http://127.0.0.1:3000
echo.
echo 按任意键打开浏览器...
pause >nul
start http://127.0.0.1:3000/search
