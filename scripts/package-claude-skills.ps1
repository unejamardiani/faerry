param(
    [string[]]$SkillNames = @()
)

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$SkillsDir = Join-Path $RepoRoot "skills"
$OutputDir = Join-Path $RepoRoot "dist/claude-desktop/skills"

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null

if ($SkillNames.Count -eq 0) {
    $SkillNames = Get-ChildItem -LiteralPath $SkillsDir -Directory |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName "SKILL.md") } |
        Sort-Object Name |
        Select-Object -ExpandProperty Name

    $CurrentPackages = @{}
    foreach ($SkillName in $SkillNames) {
        $CurrentPackages["$SkillName.zip"] = $true
    }

    Get-ChildItem -LiteralPath $OutputDir -Filter "*.zip" -File | ForEach-Object {
        if (-not $CurrentPackages.ContainsKey($_.Name)) {
            Remove-Item -LiteralPath $_.FullName -Force
            Write-Host "Removed stale package $($_.FullName)"
        }
    }
}

foreach ($SkillName in $SkillNames) {
    $SkillPath = Join-Path $SkillsDir $SkillName
    $SkillFile = Join-Path $SkillPath "SKILL.md"

    if (-not (Test-Path -LiteralPath $SkillFile)) {
        Write-Host "Skipping $SkillName: missing SKILL.md"
        continue
    }

    $TempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
    $TempSkillDir = Join-Path $TempRoot $SkillName
    New-Item -ItemType Directory -Path $TempSkillDir -Force | Out-Null
    Copy-Item -LiteralPath $SkillPath -Destination $TempRoot -Recurse

    Get-ChildItem -LiteralPath $TempRoot -Filter ".DS_Store" -Recurse -Force | Remove-Item -Force

    $OutputPath = Join-Path $OutputDir "$SkillName.zip"
    if (Test-Path -LiteralPath $OutputPath) {
        Remove-Item -LiteralPath $OutputPath -Force
    }

    Compress-Archive -Path $TempSkillDir -DestinationPath $OutputPath
    Remove-Item -LiteralPath $TempRoot -Recurse -Force

    Write-Host "Created $OutputPath"
}
