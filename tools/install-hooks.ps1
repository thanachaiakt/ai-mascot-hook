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

The editing itself is done by the exe (`--install-hooks --exe <path>`, see src/hooks.rs), so
this script and the exe can never disagree. Nothing of yours is touched: other hooks, other
settings and their order stay as they are, and only ai-mascot-hook's own hooks (any hook whose
command mentions "ai-mascot-hook" or the former "seal-pet") are added, replaced or removed.
The file is rewritten as indented JSON, and the old one is saved next to it as
settings.json.bak-<time> first.

Each hook is one plain command line, `<exe> --set <state>`, which every Claude Code
version runs (the newer exec form with a separate `args` list is not used, since older
versions ignore `args` and would start the exe with no state at all). The exe path is
written with forward slashes and without quotes, so it means the same thing to Git Bash
and to PowerShell, whichever shell Claude Code picks.

.PARAMETER Exe
The ai-mascot-hook.exe the hooks run. Defaults to target\release\ai-mascot-hook.exe in this repo.

.PARAMETER Agent
Which agents to hook: claude (the default), cursor, copilot, gemini, codex, qwen, windsurf,
several separated by commas, or all (see src/hooks.rs for the files each one uses).

.PARAMETER Remove
Take ai-mascot-hook's hooks out instead of adding them.

.EXAMPLE
powershell -ExecutionPolicy Bypass -File tools\install-hooks.ps1
#>
param(
    [string]$Exe = (Join-Path $PSScriptRoot '..\target\release\ai-mascot-hook.exe'),
    [string]$Agent = 'claude',
    [switch]$Remove
)
$ErrorActionPreference = 'Stop'

if (-not (Test-Path $Exe)) {
    throw "Not found: $Exe`nBuild it first (cargo build --release) or pass -Exe <path to ai-mascot-hook.exe>."
}
$Exe = (Resolve-Path $Exe).Path
# a path with spaces would need quoting, and Git Bash and PowerShell quote differently:
# use the 8.3 short path instead, which has none
$hookExe = $Exe
if (-not $Remove -and $Exe -match ' ') {
    $hookExe = (New-Object -ComObject Scripting.FileSystemObject).GetFile($Exe).ShortPath
    if ($hookExe -match ' ') {
        throw "The exe path has spaces and no short (8.3) name: $Exe`nCopy ai-mascot-hook.exe to a folder without spaces and pass -Exe <that path>."
    }
}

# The exe is a GUI program, so run it with its output sent to files and wait for it.
$out = [IO.Path]::GetTempFileName()
$err = [IO.Path]::GetTempFileName()
try {
    $flags = if ($Remove) { @('--remove-hooks', '--agent', $Agent) } else { @('--install-hooks', '--exe', $hookExe, '--agent', $Agent) }
    $run = Start-Process -FilePath $Exe -ArgumentList $flags -Wait -PassThru -NoNewWindow `
        -RedirectStandardOutput $out -RedirectStandardError $err
    $said = (@(Get-Content $out) + @(Get-Content $err)) -join "`n"
    if ($run.ExitCode -ne 0) {
        throw "ai-mascot-hook $($flags -join ' ') failed (exit $($run.ExitCode)): $said"
    }
    Write-Host $said
} finally {
    Remove-Item $out, $err -ErrorAction SilentlyContinue
}

if (-not $Remove) {
    # prove the exe can write the state the seal reads
    Start-Process -FilePath $Exe -ArgumentList '--set', 'idle' -Wait -NoNewWindow
    $state = Join-Path $env:LOCALAPPDATA 'ai-mascot-hook\state'
    Write-Host "Test write: $state = '$([IO.File]::ReadAllText($state))'"
    Write-Host 'Restart the agent (for Claude Code, or open /hooks) so it loads the new hooks.'
}
