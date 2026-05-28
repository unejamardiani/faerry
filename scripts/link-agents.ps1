param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path,
    [string]$HomeDir = $HOME,
    [string]$CodexHome = $(if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME ".codex" }),
    [switch]$SkipClaude,
    [switch]$SkipOpenCode,
    [switch]$SkipCodex,
    [switch]$SkipCopilotEnv
)

$ErrorActionPreference = "Stop"

$AgentsHome = Join-Path $HomeDir ".agents"
if (
    -not (Test-Path -LiteralPath (Join-Path $RepoRoot "AGENTS.md")) -or
    -not (Test-Path -LiteralPath (Join-Path $RepoRoot "skills"))
) {
    throw "Repo root does not match the expected .agents layout: $RepoRoot"
}

$Timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$script:BackupRoot = if ($env:AGENTS_BACKUP_ROOT) { $env:AGENTS_BACKUP_ROOT } else { Join-Path $HomeDir ".agents-backups/$Timestamp" }
$script:BackupCount = 0

function Ensure-Parent {
    param([string]$Path)
    $Parent = Split-Path -Parent $Path
    if ($Parent -and -not (Test-Path -LiteralPath $Parent)) {
        New-Item -ItemType Directory -Path $Parent -Force | Out-Null
    }
}

function Get-BackupDestination {
    param([string]$Path)

    if ($Path.StartsWith($HomeDir, [System.StringComparison]::OrdinalIgnoreCase)) {
        $Relative = $Path.Substring($HomeDir.Length).TrimStart('\', '/')
    } else {
        $Relative = "absolute/" + $Path.TrimStart('\', '/').Replace(':', '')
    }

    Join-Path $script:BackupRoot $Relative
}

function Backup-Target {
    param([string]$Path)

    $Destination = Get-BackupDestination -Path $Path
    Ensure-Parent -Path $Destination
    Move-Item -LiteralPath $Path -Destination $Destination
    $script:BackupCount += 1
    Write-Host "  backed up $Path -> $Destination"
}

function Get-LinkTarget {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path)) {
        return $null
    }

    $Item = Get-Item -LiteralPath $Path -Force
    if (-not ($Item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        return $null
    }

    if ($Item.LinkTarget -is [array]) {
        return $Item.LinkTarget[0]
    }

    return $Item.LinkTarget
}

function New-OrUpdateSymlink {
    param(
        [string]$Source,
        [string]$Target,
        [string]$Label
    )

    if (-not (Test-Path -LiteralPath $Source)) {
        throw "Missing source for $Label: $Source"
    }

    Ensure-Parent -Path $Target

    $CurrentTarget = Get-LinkTarget -Path $Target
    if ($CurrentTarget -eq $Source) {
        Write-Host "  ok  $Label"
        return
    }

    if (Test-Path -LiteralPath $Target) {
        Backup-Target -Path $Target
    }

    New-Item -ItemType SymbolicLink -Path $Target -Target $Source | Out-Null
    Write-Host "  link $Label -> $Target"
}

function Write-ManagedFile {
    param(
        [string]$Target,
        [string]$Content
    )

    Ensure-Parent -Path $Target

    if (Test-Path -LiteralPath $Target) {
        $Existing = Get-Content -LiteralPath $Target -Raw
        if ($Existing -eq $Content) {
            return
        }
        Backup-Target -Path $Target
    }

    Set-Content -LiteralPath $Target -Value $Content
    Write-Host "  wrote $Target"
}

function Install-SharedAgents {
    Write-Host ""
    Write-Host "Shared .agents"

    if ($RepoRoot.TrimEnd('\', '/') -eq $AgentsHome.TrimEnd('\', '/')) {
        Write-Host "  repo already lives at $AgentsHome"
        return
    }

    if (-not (Test-Path -LiteralPath $AgentsHome)) {
        New-Item -ItemType Directory -Path $AgentsHome -Force | Out-Null
    }

    New-OrUpdateSymlink -Source (Join-Path $RepoRoot "AGENTS.md") -Target (Join-Path $AgentsHome "AGENTS.md") -Label "shared AGENTS.md"
    New-OrUpdateSymlink -Source (Join-Path $RepoRoot "skills") -Target (Join-Path $AgentsHome "skills") -Label "shared skills"
}

function Install-Claude {
    if ($SkipClaude) { return }
    Write-Host ""
    Write-Host "Claude Code"

    New-OrUpdateSymlink -Source (Join-Path $AgentsHome "AGENTS.md") -Target (Join-Path $HomeDir ".claude/CLAUDE.md") -Label "Claude global context"
    New-OrUpdateSymlink -Source (Join-Path $AgentsHome "skills") -Target (Join-Path $HomeDir ".claude/skills") -Label "Claude skills"
}

function Install-OpenCode {
    if ($SkipOpenCode) { return }
    Write-Host ""
    Write-Host "OpenCode"

    $OpenCodeHome = Join-Path $HomeDir ".config/opencode"
    New-OrUpdateSymlink -Source (Join-Path $AgentsHome "AGENTS.md") -Target (Join-Path $OpenCodeHome "AGENTS.md") -Label "OpenCode global context"
    New-OrUpdateSymlink -Source (Join-Path $AgentsHome "skills") -Target (Join-Path $OpenCodeHome "skills") -Label "OpenCode skills"
}

function Install-Codex {
    if ($SkipCodex) { return }
    Write-Host ""
    Write-Host "Codex"

    New-OrUpdateSymlink -Source (Join-Path $AgentsHome "AGENTS.md") -Target (Join-Path $CodexHome "AGENTS.md") -Label "Codex global context"
    New-OrUpdateSymlink -Source (Join-Path $AgentsHome "skills") -Target (Join-Path $CodexHome "skills") -Label "Codex skills"
}

function Install-CopilotEnv {
    if ($SkipCopilotEnv) { return }
    Write-Host ""
    Write-Host "GitHub Copilot CLI"

    $SnippetPath = Join-Path $HomeDir ".config/agents/github-copilot-cli.env.ps1"
    $Content = @"
\$env:COPILOT_CUSTOM_INSTRUCTIONS_DIRS = "$AgentsHome"
\$env:COPILOT_SKILLS_DIRS = "$(Join-Path $AgentsHome "skills")"
"@

    Write-ManagedFile -Target $SnippetPath -Content $Content
    Write-Host "  dot-source $SnippetPath in shells that run github-copilot-cli"
}

Write-Host "agents repo linker"
Write-Host "repo   $RepoRoot"
Write-Host "target $HomeDir"

Install-SharedAgents
Install-Claude
Install-OpenCode
Install-Codex
Install-CopilotEnv

Write-Host ""
if ($script:BackupCount -gt 0) {
    Write-Host "Backups: $script:BackupRoot"
} else {
    Write-Host "Backups: none"
}
