param(
    [string]$BaseUrl = "http://127.0.0.1:3000",
    [int]$NSettings = 3,
    [int]$NAiNoWeb = 3,
    [int]$NAiWeb = 2,
    [int]$NSearchOnline = 2,
    [int]$TimeoutSec = 180,
    [string]$OutCsv = "logs/perf/baseline.csv"
)

$ErrorActionPreference = "Stop"

function New-ResultList {
    return [System.Collections.Generic.List[object]]::new()
}

function Add-Result {
    param(
        [object]$Rows,
        [string]$Case,
        [int]$Idx,
        [int]$Ms,
        [string]$Status,
        [string]$Err
    )
    $null = $Rows.Add([pscustomobject]@{
            case   = $Case
            idx    = $Idx
            ms     = $Ms
            status = $Status
            err    = $Err
        })
}

function Invoke-Case {
    param(
        [object]$Rows,
        [string]$CaseName,
        [string]$Uri,
        $Body,
        [int]$Count,
        [int]$TimeoutSec
    )

    for ($i = 1; $i -le $Count; $i++) {
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $status = "ok"
        $err = ""
        try {
            if ($null -eq $Body) {
                $null = Invoke-RestMethod -Uri $Uri -Method GET -TimeoutSec $TimeoutSec
            }
            else {
                $json = $Body | ConvertTo-Json -Depth 8
                $null = Invoke-RestMethod -Uri $Uri -Method POST -ContentType "application/json" -Body $json -TimeoutSec $TimeoutSec
            }
        }
        catch {
            $status = "err"
            $err = $_.Exception.Message
        }
        $sw.Stop()
        Add-Result -Rows $Rows -Case $CaseName -Idx $i -Ms ([int]$sw.ElapsedMilliseconds) -Status $status -Err $err
    }
}

function Show-Stats {
    param([object]$Rows)

    $Rows | Group-Object case | ForEach-Object {
        $grp = $_.Group
        $msSorted = $grp.ms | Sort-Object
        $avg = [math]::Round((($grp.ms | Measure-Object -Average).Average), 1)
        $n = $grp.Count
        $p95Index = [math]::Min($n - 1, [math]::Floor($n * 0.95))
        $p95 = $msSorted[$p95Index]
        $errCount = ($grp | Where-Object { $_.status -eq "err" }).Count
        "{0}: n={1}, avg={2}ms, p95={3}ms, err={4}" -f $_.Name, $n, $avg, $p95, $errCount
    }
}

$outDir = Split-Path -Parent $OutCsv
if (-not [string]::IsNullOrWhiteSpace($outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$rows = [System.Collections.Generic.List[object]]::new()
if ($null -eq $rows) {
    throw "result list init failed"
}

Invoke-Case -Rows $rows -CaseName "settings_get" -Uri "$BaseUrl/api/settings" -Body $null -Count $NSettings -TimeoutSec $TimeoutSec
Invoke-Case -Rows $rows -CaseName "ai_chat_no_web" -Uri "$BaseUrl/api/ai/chat" -Body @{
    message    = "perf no web"
    web_search = $false
    history    = @()
} -Count $NAiNoWeb -TimeoutSec $TimeoutSec
Invoke-Case -Rows $rows -CaseName "ai_chat_web" -Uri "$BaseUrl/api/ai/chat" -Body @{
    message    = "perf with web"
    web_search = $true
    history    = @()
} -Count $NAiWeb -TimeoutSec $TimeoutSec
Invoke-Case -Rows $rows -CaseName "search_online" -Uri "$BaseUrl/api/search/online" -Body @{
    query       = "wang qing zhi"
    page        = 1
    page_size   = 5
    search_type = "keyword"
    region      = "auto"
} -Count $NSearchOnline -TimeoutSec $TimeoutSec

$rows | Export-Csv -NoTypeInformation -Encoding UTF8 $OutCsv
Show-Stats -Rows $rows
