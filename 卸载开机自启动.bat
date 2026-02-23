@echo off
chcp 65001 >nul
title 卸载 Patent Hub 开机自启动
echo ========================================
echo    卸载 Patent Hub 开机自启动
echo ========================================
echo.

set "STARTUP_FOLDER=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup"
set "SHORTCUT=%STARTUP_FOLDER%\Patent Hub.lnk"

if exist "%SHORTCUT%" (
    echo 正在删除启动快捷方式...
    del "%SHORTCUT%"
    echo ✓ 卸载成功！
    echo.
    echo Patent Hub 已从开机自启动中移除
) else (
    echo 未找到开机自启动项
)

echo.
echo 按任意键退出...
pause >nul
