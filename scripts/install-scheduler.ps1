# 安装 InnoForge 自审查计划任务 / Install self-review scheduled task
# 以管理员权限运行 / Run as Administrator

$TaskName    = "InnoForge-SelfReview"
$ProjectRoot = Split-Path $PSScriptRoot -Parent
$ScriptPath  = Join-Path $ProjectRoot "scripts\self-review.ps1"
$LogPath     = Join-Path $ProjectRoot "docs\self-review\scheduler.log"

# 每30分钟触发，脚本内部判断是否真正执行
$trigger = New-ScheduledTaskTrigger -RepetitionInterval (New-TimeSpan -Minutes 30) -Once -At (Get-Date)

$action = New-ScheduledTaskAction `
    -Execute "powershell.exe" `
    -Argument "-NonInteractive -ExecutionPolicy Bypass -File `"$ScriptPath`" >> `"$LogPath`" 2>&1" `
    -WorkingDirectory $ProjectRoot

$settings = New-ScheduledTaskSettingsSet `
    -ExecutionTimeLimit (New-TimeSpan -Minutes 20) `
    -MultipleInstances IgnoreNew `
    -StartWhenAvailable

Register-ScheduledTask `
    -TaskName $TaskName `
    -Trigger $trigger `
    -Action $action `
    -Settings $settings `
    -RunLevel Highest `
    -Force | Out-Null

Write-Host "✅ 计划任务已创建: $TaskName" -ForegroundColor Green
Write-Host "   每30分钟检测一次，闲置超过30分钟时自动运行自审查"
Write-Host "   日志: $LogPath"
Write-Host ""
Write-Host "管理任务 / Manage task:"
Write-Host "  查看:  Get-ScheduledTask -TaskName $TaskName"
Write-Host "  立即运行: Start-ScheduledTask -TaskName $TaskName"
Write-Host "  禁用:  Disable-ScheduledTask -TaskName $TaskName"
Write-Host "  删除:  Unregister-ScheduledTask -TaskName $TaskName -Confirm:`$false"
