@echo off
chcp 65001 >nul
title 打包 Patent Hub 发布版
echo ========================================
echo    打包 Patent Hub 发布版
echo ========================================
echo.

set "RELEASE_DIR=Patent-Hub-Release"
set "VERSION=v1.0.0"

echo 正在创建发布目录...
if exist "%RELEASE_DIR%" rmdir /s /q "%RELEASE_DIR%"
mkdir "%RELEASE_DIR%"

echo 复制可执行文件...
copy "target\release\patent-hub.exe" "%RELEASE_DIR%\" >nul

echo 复制模板文件...
xcopy "templates" "%RELEASE_DIR%\templates\" /E /I /Y >nul

echo 复制静态资源...
xcopy "static" "%RELEASE_DIR%\static\" /E /I /Y >nul

echo 复制启动脚本...
copy "启动服务器.bat" "%RELEASE_DIR%\" >nul
copy "开机自启动.vbs" "%RELEASE_DIR%\" >nul
copy "安装开机自启动.bat" "%RELEASE_DIR%\" >nul
copy "卸载开机自启动.bat" "%RELEASE_DIR%\" >nul

echo 复制配置文件...
copy ".env.example" "%RELEASE_DIR%\.env" >nul

echo 复制说明文档...
copy "README.md" "%RELEASE_DIR%\" >nul

echo 创建使用说明...
(
echo ========================================
echo    Patent Hub 专利检索系统 %VERSION%
echo ========================================
echo.
echo 【快速开始】
echo 1. 双击"启动服务器.bat"启动系统
echo 2. 浏览器会自动打开 http://127.0.0.1:3000
echo 3. 开始搜索专利！
echo.
echo 【开机自启动】
echo - 安装：双击"安装开机自启动.bat"
echo - 卸载：双击"卸载开机自启动.bat"
echo.
echo 【配置 API 密钥】
echo 编辑 .env 文件，填入你的 API 密钥：
echo - SERPAPI_KEY: 用于在线搜索（必需）
echo - AI_API_KEY: 用于 AI 分析（可选）
echo.
echo 【功能特性】
echo ✓ 在线专利搜索（Google Patents）
echo ✓ 搜索历史记录
echo ✓ 日期和国家筛选
echo ✓ 统计分析图表
echo ✓ 导出 Excel
echo ✓ AI 智能分析
echo ✓ 专利对比
echo ✓ 相似专利推荐
echo ✓ 文件对比分析
echo.
echo 【系统要求】
echo - Windows 7/8/10/11
echo - 无需安装其他软件
echo - 建议配置 API 密钥以使用完整功能
echo.
echo 【常见问题】
echo Q: 搜索没有结果？
echo A: 请确保已配置 SERPAPI_KEY 并连接网络
echo.
echo Q: 如何停止服务器？
echo A: 关闭命令行窗口或按 Ctrl+C
echo.
echo Q: 可以在其他电脑上使用吗？
echo A: 可以！直接复制整个文件夹到其他电脑
echo.
echo 【技术支持】
echo 详细文档请查看 README.md
echo.
echo ========================================
) > "%RELEASE_DIR%\使用说明.txt"

echo.
echo ✓ 打包完成！
echo.
echo 发布包位置: %RELEASE_DIR%\
echo.
echo 【分发方式】
echo 1. 压缩 %RELEASE_DIR% 文件夹为 ZIP
echo 2. 发送给其他人
echo 3. 对方解压后双击"启动服务器.bat"即可使用
echo.
echo 正在打开发布目录...
explorer "%RELEASE_DIR%"
echo.
echo 按任意键退出...
pause >nul
