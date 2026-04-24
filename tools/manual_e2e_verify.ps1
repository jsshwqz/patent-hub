param(
    [string]$BaseUrl = "http://127.0.0.1:3000",
    [string]$OutDir = "docs/visible-e2e-runs"
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$target = Join-Path $scriptDir "manual_e2e_verify_ascii.ps1"

if (-not (Test-Path $target)) {
    throw "missing script: $target"
}

powershell -NoProfile -ExecutionPolicy Bypass -File $target -BaseUrl $BaseUrl -OutDir $OutDir
exit $LASTEXITCODE
