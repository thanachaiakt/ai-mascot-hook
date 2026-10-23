<#
.SYNOPSIS
Point Claude Code's hooks at ai-mascot-hook, so the seal follows what Claude is doing.

.DESCRIPTION
Adds hooks to ~/.claude/settings.json that run `ai-mascot-hook.exe --set <state>`:

    SessionStart       -> idle
    UserPromptSubmit   -> loading     (Claude is thinking)
    PreToolUse         -> working     (a tool is running)
    PostToolUse        -> working
    PermissionRequest  -> asking      (Claude needs your OK)
    Notification       -> asking      (permission prompt, waiting for input, MCP question)
    Stop               -> completed
    SessionEnd         -> idle

Each hook is one plain command line, `<exe> --set <state>`, which every Claude Code
version runs (the newer exec form with a separate `args` list is not used, since older
versions ignore `args` and would start the exe with no state at all). The exe path is
written with forward slashes and without quotes, so it means the same thing to Git Bash
and to PowerShell, whichever shell Claude Code picks. Earlier ai-mascot-hook hooks (any hook whose command mentions
"ai-mascot-hook" or the former "seal-pet") are replaced; every other hook and setting is kept. The old file is saved
next to it as settings.json.bak-<time> first.

.PARAMETER Exe
The ai-mascot-hook.exe the hooks run. Defaults to target\release\ai-mascot-hook.exe in this repo.

.PARAMETER Remove
Take ai-mascot-hook's hooks out instead of adding them.

.EXAMPLE
powershell -ExecutionPolicy Bypass -File tools\install-hooks.ps1
#>
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\release\ai-mascot-hook.exe'),
    [switch]$Remove
)
$ErrorActionPreference = 'Stop'

$settingsPath = Join-Path $env:USERPROFILE '.claude\settings.json'

if (-not $Remove) {
    if (-not (Test-Path $Exe)) {
        throw "Not found: $Exe`nBuild it first (cargo build --release) or pass -Exe <path to ai-mascot-hook.exe>."
    }
    $Exe = (Resolve-Path $Exe).Path
    # a path with spaces would need quoting, and Git Bash and PowerShell quote differently:
    # use the 8.3 short path instead, which has none
    if ($Exe -match ' ') {
        $short = (New-Object -ComObject Scripting.FileSystemObject).GetFile($Exe).ShortPath
        if ($short -match ' ') {
            throw "The exe path has spaces and no short (8.3) name: $Exe`nCopy ai-mascot-hook.exe to a folder without spaces and pass -Exe <that path>."
        }
        $Exe = $short
    }
    $ExeCmd = $Exe -replace '\\', '/'
}

# ---- read the current settings ---------------------------------------------------
if (Test-Path $settingsPath) {
    $raw = [IO.File]::ReadAllText($settingsPath)
    $settings = if ($raw.Trim()) { $raw | ConvertFrom-Json } else { New-Object PSObject }
    $backup = "$settingsPath.bak-$(Get-Date -Format yyyyMMdd-HHmmss)"
    Copy-Item $settingsPath $backup
    Write-Host "Backed up settings to $backup"
} else {
    New-Item -ItemType Directory -Force -Path (Split-Path $settingsPath) | Out-Null
    $settings = New-Object PSObject
}
if (-not ($settings.PSObject.Properties.Name -contains 'hooks') -or $null -eq $settings.hooks) {
    $settings | Add-Member -NotePropertyName hooks -NotePropertyValue (New-Object PSObject) -Force
}
$hooks = $settings.hooks

function Test-SealPet($handler) {
    ("$($handler.command) $($handler.args -join ' ')") -match 'ai-mascot-hook|seal-pet'
}

# ---- drop any earlier ai-mascot-hook hooks -------------------------------------------------
foreach ($event in @($hooks.PSObject.Properties.Name)) {
    $groups = @()
    foreach ($group in @($hooks.$event)) {
        $kept = @(@($group.hooks) | Where-Object { -not (Test-SealPet $_) })
        if ($kept.Count -gt 0) {
            $group.hooks = $kept
            $groups += $group
        }
    }
    if ($groups.Count -gt 0) { $hooks.$event = $groups } else { $hooks.PSObject.Properties.Remove($event) }
}

# ---- add ours ------------------------------------------------------------------------
if (-not $Remove) {
    $plan = [ordered]@{
        SessionStart      = @{ state = 'idle' }
        UserPromptSubmit  = @{ state = 'loading' }
        PreToolUse        = @{ state = 'working'; matcher = '*' }
        PostToolUse       = @{ state = 'working'; matcher = '*' }
        PermissionRequest = @{ state = 'asking'; matcher = '*' }
        Notification      = @{ state = 'asking'; matcher = 'permission_prompt|idle_prompt|elicitation_dialog' }
        Stop              = @{ state = 'completed' }
        SessionEnd        = @{ state = 'idle' }
    }
    foreach ($event in $plan.Keys) {
        $step = $plan[$event]
        $handler = [ordered]@{ type = 'command'; command = "$ExeCmd --set $($step.state)"; timeout = 10 }
        $group = [ordered]@{}
        if ($step.matcher) { $group.matcher = $step.matcher }
        $group.hooks = @($handler)
        $existing = if ($hooks.PSObject.Properties.Name -contains $event) { @($hooks.$event) } else { @() }
        $hooks | Add-Member -NotePropertyName $event -NotePropertyValue (@($existing) + (New-Object PSObject -Property $group)) -Force
    }
}

# ---- write it back: UTF-8 without a BOM ------------------------------------------------
$json = $settings | ConvertTo-Json -Depth 32
[IO.File]::WriteAllText($settingsPath, $json, (New-Object Text.UTF8Encoding $false))

if ($Remove) {
    Write-Host "Removed ai-mascot-hook's hooks from $settingsPath"
} else {
    # prove the exe can write the state the seal reads (it is a GUI exe: wait for it explicitly)
    Start-Process -FilePath $Exe -ArgumentList '--set', 'idle' -Wait -NoNewWindow
    $state = Join-Path $env:LOCALAPPDATA 'ai-mascot-hook\state'
    Write-Host "Hooks installed in $settingsPath -> $Exe"
    Write-Host "Test write: $state = '$([IO.File]::ReadAllText($state))'"
    Write-Host 'Restart Claude Code (or open /hooks) so it loads the new hooks.'
}
