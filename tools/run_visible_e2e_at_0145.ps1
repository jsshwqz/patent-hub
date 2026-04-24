param(
    [string]$StartAt = "01:45",
    [string]$RepoRoot = "D:\test\patent-hub-backup"
)

$ErrorActionPreference = "Stop"

$outRoot = Join-Path $RepoRoot "docs\visible-e2e-runs"
New-Item -ItemType Directory -Path $outRoot -Force | Out-Null

$parts = $StartAt.Split(":")
if ($parts.Count -ne 2) {
    throw "Invalid StartAt format. Expected HH:mm"
}
$h = [int]$parts[0]
$m = [int]$parts[1]

$now = Get-Date
$target = Get-Date -Year $now.Year -Month $now.Month -Day $now.Day -Hour $h -Minute $m -Second 0
if ($target -le $now) {
    $target = $target.AddDays(1)
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$runDir = Join-Path $outRoot ("run-" + $stamp + "-auto")
New-Item -ItemType Directory -Path $runDir -Force | Out-Null
$statusLog = Join-Path $runDir "scheduler.log"
$latestPtr = Join-Path $outRoot "LATEST_RUN_PATH.txt"
Set-Content -Path $latestPtr -Value $runDir -Encoding UTF8

function LogLine([string]$msg) {
    $line = ("[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss"), $msg)
    $line | Tee-Object -FilePath $statusLog -Append
}

LogLine "Scheduler started. Target time: $($target.ToString('yyyy-MM-dd HH:mm:ss'))"

while ((Get-Date) -lt $target) {
    Start-Sleep -Seconds 5
}

LogLine "Target time reached. Starting visible E2E run."

$runner = Join-Path $RepoRoot "tools\visible_e2e_test.py"
if (-not (Test-Path $runner)) {
    LogLine "Runner script not found: $runner"
    exit 1
}

$python = "python"
$runOut = Join-Path $runDir "result"
New-Item -ItemType Directory -Path $runOut -Force | Out-Null

LogLine "Running: $python $runner --out $runOut --versions v0.5.0,v0.5.3"
& $python $runner --out $runOut --versions "v0.5.0,v0.5.3" 2>&1 | Tee-Object -FilePath (Join-Path $runDir "runner.log") -Append
$code = $LASTEXITCODE

if ($code -eq 0) {
    LogLine "Visible E2E completed (exit=0). Output: $runOut"
} else {
    LogLine "Visible E2E failed (exit=$code). Check runner.log"
}

exit $code
