param(
    [string]$BaseUrl = "http://127.0.0.1:3000",
    [string]$OutDir = "docs/visible-e2e-runs"
)

$ErrorActionPreference = "Stop"

function New-Case([string]$Name) {
    [pscustomobject]@{ name = $Name; ok = $false; detail = ""; ms = 0 }
}

function Invoke-JsonPost([string]$Url, [object]$Body, [int]$TimeoutSec = 60) {
    $json = $Body | ConvertTo-Json -Depth 10
    Invoke-RestMethod -Uri $Url -Method POST -ContentType "application/json" -Body $json -TimeoutSec $TimeoutSec
}

$ts = Get-Date -Format "yyyyMMdd_HHmmss"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$jsonPath = Join-Path $OutDir "manual-e2e-$ts.json"
$mdPath = Join-Path $OutDir "manual-e2e-$ts.md"
$cases = [System.Collections.Generic.List[object]]::new()

# A. page reachability
foreach ($p in @("/", "/search", "/idea", "/compare", "/settings", "/ai")) {
    $c = New-Case "page$p"
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $r = Invoke-WebRequest -Uri ($BaseUrl + $p) -UseBasicParsing -TimeoutSec 20
        $c.ok = ($r.StatusCode -eq 200 -and $r.Content.Length -gt 100)
        $c.detail = "status=$($r.StatusCode) len=$($r.Content.Length)"
    } catch {
        $c.detail = $_.Exception.Message
    }
    $sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds
    $cases.Add($c) | Out-Null
}

# B1 idea submit + list
$ideaId = ""
$c = New-Case "idea_submit"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $submit = Invoke-JsonPost "$BaseUrl/api/idea/submit" @{
        title = "E2E-$ts"
        description = "E2E auto check"
        input_type = "text"
    } 60
    if ($submit.status -eq "ok" -and $submit.id) {
        $ideaId = [string]$submit.id
        $c.ok = $true
        $c.detail = "id=$ideaId"
    } else {
        $c.detail = "invalid payload"
    }
} catch { $c.detail = $_.Exception.Message }
$sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds; $cases.Add($c) | Out-Null

$c = New-Case "idea_list_contains"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $list = Invoke-RestMethod -Uri "$BaseUrl/api/idea/list" -Method GET -TimeoutSec 30
    $found = $false
    if ($list.ideas) { foreach ($it in $list.ideas) { if ($it.id -eq $ideaId) { $found = $true; break } } }
    $c.ok = $found
    $c.detail = "found=$found"
} catch { $c.detail = $_.Exception.Message }
$sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds; $cases.Add($c) | Out-Null

# B2 report
$c = New-Case "idea_report"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $report = Invoke-RestMethod -Uri "$BaseUrl/api/idea/$ideaId/report" -Method GET -TimeoutSec 60
    $payload = $report | ConvertTo-Json -Depth 6
    $c.ok = ($payload.Length -gt 50)
    $c.detail = "len=$($payload.Length)"
} catch { $c.detail = $_.Exception.Message }
$sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds; $cases.Add($c) | Out-Null

# B3 chat + messages
$c = New-Case "idea_chat_and_messages"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $chat = Invoke-JsonPost "$BaseUrl/api/idea/$ideaId/chat" @{ message = "one line risk summary" } 90
    $msgs = Invoke-RestMethod -Uri "$BaseUrl/api/idea/$ideaId/messages" -Method GET -TimeoutSec 30
    $replyText = ""
    if ($chat.reply) { $replyText = [string]$chat.reply }
    if (-not $replyText -and $chat.message -and $chat.message.content) { $replyText = [string]$chat.message.content }
    $okChat = ($replyText.Length -gt 0)
    $rateLimited = $false
    if ($chat.error -and ([string]$chat.error).Contains("频率限制")) {
        $rateLimited = $true
    }
    $okMsgs = ($msgs.messages -and $msgs.messages.Count -ge 1)
    $c.ok = (($okChat -or $rateLimited) -and $okMsgs)
    $c.detail = "reply=$okChat rate_limited=$rateLimited messages=$okMsgs"
} catch { $c.detail = $_.Exception.Message }
$sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds; $cases.Add($c) | Out-Null

# B5 search keyword
$c = New-Case "search_keyword"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $r = Invoke-JsonPost "$BaseUrl/api/search/online" @{
        query = "battery thermal management"
        page = 1
        page_size = 10
        search_type = "keyword"
        region = "intl"
    } 90
    $count = 0; if ($r.patents) { $count = $r.patents.Count }
    $c.ok = ($r.total -ne $null)
    $c.detail = "total=$($r.total) count=$count source=$($r.source)"
} catch { $c.detail = $_.Exception.Message }
$sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds; $cases.Add($c) | Out-Null

# B5 search by patent number
$c = New-Case "search_patent_number"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $r = Invoke-JsonPost "$BaseUrl/api/search/online" @{
        query = "US10000000"
        page = 1
        page_size = 10
        search_type = "patent_number"
        region = "intl"
    } 90
    $count = 0; if ($r.patents) { $count = $r.patents.Count }
    $c.ok = ($r.total -ne $null)
    $c.detail = "total=$($r.total) count=$count source=$($r.source)"
} catch { $c.detail = $_.Exception.Message }
$sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds; $cases.Add($c) | Out-Null

# B6 compare with two text items
$c = New-Case "compare_text_items"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $cmp = Invoke-JsonPost "$BaseUrl/api/ai/compare" @{
        items = @(
            @{ type = "text"; title = "SampleA"; content = "Battery pack thermal management with closed-loop control." },
            @{ type = "text"; title = "SampleB"; content = "Battery thermal runaway early warning by multi-sensor fusion." }
        )
    } 120
    $len = 0; if ($cmp.content) { $len = ([string]$cmp.content).Length }
    $c.ok = ($len -gt 10)
    $c.detail = "content_len=$len"
} catch { $c.detail = $_.Exception.Message }
$sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds; $cases.Add($c) | Out-Null

# B4 delete idea
$c = New-Case "idea_delete"
$sw = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $del = Invoke-JsonPost "$BaseUrl/api/idea/$ideaId/delete" @{} 30
    $list2 = Invoke-RestMethod -Uri "$BaseUrl/api/idea/list" -Method GET -TimeoutSec 30
    $found = $false
    if ($list2.ideas) { foreach ($it in $list2.ideas) { if ($it.id -eq $ideaId) { $found = $true; break } } }
    $c.ok = (-not $found)
    $c.detail = "delete_status=$($del.status) exists_after=$found"
} catch { $c.detail = $_.Exception.Message }
$sw.Stop(); $c.ms = [int]$sw.ElapsedMilliseconds; $cases.Add($c) | Out-Null

# C non-functional
function Run-CmdCase([string]$Name, [scriptblock]$Action) {
    $n = New-Case $Name
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        & $Action
        $code = $LASTEXITCODE
        if ($null -eq $code) { $code = 0 }
        $n.ok = ($code -eq 0)
        $n.detail = "exit=$code"
    } catch {
        $n.ok = $false
        $n.detail = $_.Exception.Message
    }
    $sw.Stop()
    $n.ms = [int]$sw.ElapsedMilliseconds
    return $n
}

$cases.Add((Run-CmdCase "cargo_fmt" { cargo fmt --all -- --check | Out-Null })) | Out-Null
$cases.Add((Run-CmdCase "cargo_test_tests" { cargo test --tests -q | Out-Null })) | Out-Null
$cases.Add((Run-CmdCase "cargo_clippy_tests" { cargo clippy --tests -- -D warnings | Out-Null })) | Out-Null

$pass = ($cases | Where-Object { $_.ok }).Count
$total = $cases.Count
$fail = $total - $pass

$report = [pscustomobject]@{
    generated_at = (Get-Date).ToString("s")
    base_url = $BaseUrl
    total = $total
    pass = $pass
    fail = $fail
    items = $cases
}
$report | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 $jsonPath

$md = [System.Collections.Generic.List[string]]::new()
$md.Add("# manual e2e run $ts") | Out-Null
$md.Add("") | Out-Null
$md.Add("- base: $BaseUrl") | Out-Null
$md.Add("- pass: $pass/$total fail: $fail") | Out-Null
$md.Add("") | Out-Null
$md.Add("| case | result | ms | detail |") | Out-Null
$md.Add("|---|---|---:|---|") | Out-Null
foreach ($it in $cases) {
    $res = if ($it.ok) { "PASS" } else { "FAIL" }
    $detail = ([string]$it.detail).Replace("|", "/").Replace("`r", " ").Replace("`n", " ")
    $md.Add("| $($it.name) | $res | $($it.ms) | $detail |") | Out-Null
}
($md -join "`n") | Set-Content -Encoding UTF8 $mdPath

Write-Output "JSON=$jsonPath"
Write-Output "MD=$mdPath"
Write-Output "PASS=$pass/$total"
