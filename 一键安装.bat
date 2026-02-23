@echo off
chcp 65001 >nul
title Patent Hub 一键安装
color 0A
echo.
echo    ╔════════════════════════════════════════╗
echo    ║   Patent Hub 专利检索系统 v1.0.0      ║
echo    ║   一键安装程序                         ║
echo    ╚════════════════════════════════════════╝
echo.
echo    正在检查系统环境...
echo.

REM 检查是否有可执行文件
if not exist "target\release\patent-hub.exe" (
    echo    ✗ 错误：未找到可执行文件
    echo.
    echo    请先运行以下命令编译项目：
    echo    cargo build --release
    echo.
    goto :end
)

echo    ✓ 可执行文件检查通过
echo.

REM 检查配置文件
if not exist ".env" (
    echo    正在创建配置文件...
    copy ".env.example" ".env" >nul
    echo    ✓ 配置文件已创建
    echo.
    echo    ⚠ 提示：请编辑 .env 文件配置 API 密钥
    echo.
) else (
    echo    ✓ 配置文件已存在
    echo.
)

REM 询问是否安装开机自启动
echo    是否安装开机自启动？
echo    [Y] 是    [N] 否
echo.
choice /C YN /N /M "    请选择: "
if errorlevel 2 goto :skip_startup
if errorlevel 1 goto :install_startup

:install_startup
echo.
echo    正在安装开机自启动...
call "安装开机自启动.bat" >nul 2>&1
echo    ✓ 开机自启动已安装
echo.
goto :start_server

:skip_startup
echo.
echo    已跳过开机自启动安装
echo.

:start_server
echo    是否立即启动服务器？
echo    [Y] 是    [N] 否
echo.
choice /C YN /N /M "    请选择: "
if errorlevel 2 goto :finish
if errorlevel 1 goto :do_start

:do_start
echo.
echo    正在启动服务器...
echo.
start "" "启动服务器.bat"
timeout /t 2 >nul
goto :finish

:finish
echo.
echo    ╔════════════════════════════════════════╗
echo    ║   安装完成！                           ║
echo    ╚════════════════════════════════════════╝
echo.
echo    【使用方法】
echo    • 双击"启动服务器.bat"启动系统
echo    • 访问 http://127.0.0.1:3000
echo.
echo    【配置 API】
echo    • 编辑 .env 文件
echo    • 填入 SERPAPI_KEY（必需）
echo    • 填入 AI_API_KEY（可选）
echo.
echo    【管理自启动】
echo    • 安装：运行"安装开机自启动.bat"
echo    • 卸载：运行"卸载开机自启动.bat"
echo.

:end
echo    按任意键退出...
pause >nul
