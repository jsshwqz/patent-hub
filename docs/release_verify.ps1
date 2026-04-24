param(
    [string]$Owner = "jsshwqz",
    [string]$Repo = "innoforge",
    [string]$Tag = ""
)

$ErrorActionPreference = "Stop"

function Write-Check([string]$name, [bool]$ok, [string]$detail) {
    $flag = if ($ok) { "PASS" } else { "FAIL" }
    Write-Host ("[{0}] {1} - {2}" -f $flag, $name, $detail)
}

Write-Host "== 本地静态检查 =="
cargo fmt --all
Write-Check "cargo fmt" ($LASTEXITCODE -eq 0) "格式检查"
if ($LASTEXITCODE -ne 0) { exit 1 }

cargo test --tests
Write-Check "cargo test --tests" ($LASTEXITCODE -eq 0) "测试检查"
if ($LASTEXITCODE -ne 0) { exit 1 }

cargo clippy --tests -- -D warnings
Write-Check "cargo clippy" ($LASTEXITCODE -eq 0) "静态检查"
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host ""
Write-Host "== 双端一致性检查 =="
$head = (git rev-parse HEAD).Trim()
$gh = (git ls-remote origin refs/heads/main).Split("`t")[0]
$ge = (git ls-remote gitee refs/heads/main).Split("`t")[0]
Write-Check "HEAD=origin/main" ($head -eq $gh) "$head"
Write-Check "HEAD=gitee/main" ($head -eq $ge) "$head"

if ($Tag -ne "") {
    Write-Host ""
    Write-Host "== Release 资产检查 =="
    $url = "https://api.github.com/repos/$Owner/$Repo/releases/tags/$Tag"
    $rel = Invoke-RestMethod -Uri $url
    $assetCount = @($rel.assets).Count
    Write-Check "GitHub Release Assets" ($assetCount -ge 5) "tag=$Tag, assets=$assetCount"
}

Write-Host ""
Write-Host "发布前自动核验完成。"
