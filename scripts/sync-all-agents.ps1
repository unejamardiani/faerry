param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
    [string]$HomeDir = $HOME,
    [string]$CodexHome = $(if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME ".codex" }),
    [switch]$SkipClaude,
    [switch]$SkipOpenCode,
    [switch]$SkipCodex,
    [switch]$SkipCopilotEnv,
    [switch]$SkipClaudePackages,
    [switch]$WithCoworkLive,
    [switch]$DryRunCowork,
    [switch]$WithMcps,
    [switch]$DryRunMcps
)

$ErrorActionPreference = "Stop"

$ScriptDir = Join-Path $RepoRoot "scripts"

Write-Host "Faerry workspace sync"
Write-Host "workspace $RepoRoot"

$LinkArgs = @(
    "-RepoRoot", $RepoRoot,
    "-HomeDir", $HomeDir,
    "-CodexHome", $CodexHome
)

if ($SkipClaude) { $LinkArgs += "-SkipClaude" }
if ($SkipOpenCode) { $LinkArgs += "-SkipOpenCode" }
if ($SkipCodex) { $LinkArgs += "-SkipCodex" }
if ($SkipCopilotEnv) { $LinkArgs += "-SkipCopilotEnv" }

& (Join-Path $ScriptDir "link-agents.ps1") @LinkArgs

if (-not $SkipClaudePackages) {
    Write-Host ""
    Write-Host "Claude Desktop skill packages"
    & (Join-Path $ScriptDir "package-claude-skills.ps1")
}

if ($WithCoworkLive -or $DryRunCowork) {
    Write-Host ""
    Write-Host "Claude/Cowork live skill workspaces"
    $CoworkArgs = @()
    if ($DryRunCowork) { $CoworkArgs += "--dry-run" }
    node (Join-Path $ScriptDir "sync-claude-cowork-skills.mjs") @CoworkArgs
}

if ($WithMcps -or $DryRunMcps) {
    Write-Host ""
    Write-Host "MCP servers"
    $McpArgs = @()
    if ($DryRunMcps) { $McpArgs += "--dry-run" }
    node (Join-Path $ScriptDir "sync-mcps.mjs") @McpArgs
}

Write-Host ""
Write-Host "Done."
