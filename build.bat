@echo off
chcp 65001 >nul
title Patent Hub 编译
echo ========================================
echo    Patent Hub 编译脚本
echo ========================================
echo.
echo 正在编译 Release 版本...
echo 预计需要 1-2 分钟
echo.

cd /d "%~dp0"
cargo build --release

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ========================================
    echo 编译成功！
    echo ========================================
    echo.
    echo 运行 start.bat 启动服务器
    echo.
) else (
    echo.
    echo ========================================
    echo 编译失败！
    echo ========================================
    echo.
)

pause
