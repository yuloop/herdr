# installed by herdr
# managed by herdr; reinstalling or updating the integration overwrites this file.
# add custom hooks beside this file instead of editing it.
# HERDR_INTEGRATION_ID=qwen
# HERDR_INTEGRATION_VERSION=1

param([string]$Action = "")

if (@("session", "working", "blocked", "idle", "release") -notcontains $Action) { exit 0 }
if ($env:HERDR_ENV -ne "1") { exit 0 }
if ([string]::IsNullOrWhiteSpace($env:HERDR_PANE_ID)) { exit 0 }

$inputText = [Console]::In.ReadToEnd()
try {
    $payload = if ([string]::IsNullOrWhiteSpace($inputText)) { $null } else { $inputText | ConvertFrom-Json }
} catch {
    $payload = $null
}

if ($null -ne $payload -and -not [string]::IsNullOrWhiteSpace($payload.agent_id)) { exit 0 }

$sessionId = if ($null -ne $payload -and $payload.session_id -is [string] -and -not [string]::IsNullOrWhiteSpace($payload.session_id)) {
    $payload.session_id
} else {
    $null
}
$seq = [DateTime]::UtcNow.Ticks

function Invoke-Herdr([string[]]$Arguments) {
    try {
        & herdr @Arguments 2>$null | Out-Null
    } catch {
    }
}

if ($Action -eq "session") {
    if ([string]::IsNullOrWhiteSpace($sessionId)) { exit 0 }
    $sessionArgs = @(
        "pane", "report-agent-session", $env:HERDR_PANE_ID,
        "--source", "herdr:qwen",
        "--agent", "qwen",
        "--agent-session-id", "$sessionId",
        "--seq", "$seq"
    )
    if ($null -ne $payload -and $payload.source -is [string] -and -not [string]::IsNullOrWhiteSpace($payload.source)) {
        $sessionArgs += @("--session-start-source", "$($payload.source)")
    }
    Invoke-Herdr $sessionArgs
    Invoke-Herdr @(
        "pane", "report-agent", $env:HERDR_PANE_ID,
        "--source", "herdr:qwen",
        "--agent", "qwen",
        "--state", "idle",
        "--agent-session-id", "$sessionId",
        "--seq", "$($seq + 1)"
    )
} elseif ($Action -eq "release") {
    Invoke-Herdr @(
        "pane", "release-agent", $env:HERDR_PANE_ID,
        "--source", "herdr:qwen",
        "--agent", "qwen",
        "--seq", "$seq"
    )
} else {
    $stateArgs = @(
        "pane", "report-agent", $env:HERDR_PANE_ID,
        "--source", "herdr:qwen",
        "--agent", "qwen",
        "--state", "$Action",
        "--seq", "$seq"
    )
    if (-not [string]::IsNullOrWhiteSpace($sessionId)) {
        $stateArgs += @("--agent-session-id", "$sessionId")
    }
    Invoke-Herdr $stateArgs
}
