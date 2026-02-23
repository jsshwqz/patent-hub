@echo off
chcp 65001 >nul
title 安装 Patent Hub 开机自启动
echo ========================================
echo    安装 Patent Hub 开机自启动
echo ========================================
echo.

set "STARTUP_FOLDER=%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup"
set "VBS_FILE=%~dp0开机自启动.vbs"
set "SHORTCUT=%STARTUP_FOLDER%\Patent Hub.lnk"

echo 正在创建启动快捷方式...
echo.

powershell -Command "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut('%SHORTCUT%'); $Shortcut.TargetPath = '%VBS_FILE%'; $Shortcut.WorkingDirectory = '%~dp0'; $Shortcut.Description = 'Patent Hub 专利检索系统'; $Shortcut.Save()"

if exist "%SHORTCUT%" (
    echo ✓ 安装成功！
    echo.
    echo Patent Hub 将在下次开机时自动启动
    echo.
    echo 快捷方式位置: %SHORTCUT%
) else (
    echo ✗ 安装失败
)

echo.
echo 按任意键退出...
pause >nul
