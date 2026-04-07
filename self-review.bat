@echo off
rem InnoForge 自审查快捷启动 / Self-Review Quick Launch
cd /d "%~dp0"
powershell -ExecutionPolicy Bypass -File scripts\self-review.ps1 -Force
pause
