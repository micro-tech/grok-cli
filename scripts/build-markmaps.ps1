# scripts/build-markmaps.ps1
# Rebuilds all Markmap HTML files from .mmd sources
# Prefers globally installed 'markmap' when available, falls back to npx.
# Usage: .\scripts\build-markmaps.ps1

$ErrorActionPreference = "Continue"
$markmapDir = ".doc/markmap"

if (-not (Test-Path $markmapDir)) {
    Write-Host "No .doc/markmap directory found." -ForegroundColor Yellow
    exit 0
}

$mmdFiles = Get-ChildItem -Path $markmapDir -Filter "*.mmd" | Sort-Object Name

if ($mmdFiles.Count -eq 0) {
    Write-Host "No .mmd files found." -ForegroundColor Yellow
    exit 0
}

$count = $mmdFiles.Count

# Detect if we were called from cargo (build.rs) — be less noisy
$isCargoBuild = $env:CARGO_MANIFEST_DIR -ne $null -or $env:CARGO_PKG_NAME -ne $null

if (-not $isCargoBuild) {
    Write-Host "Found $count Markmap file(s)..." -ForegroundColor Cyan
    Write-Host ""
}

# Detect preferred markmap command
$useMarkmap = $false
$markmapCmd = $null

# 1. Check for global 'markmap' in PATH
if (Get-Command markmap -ErrorAction SilentlyContinue) {
    $useMarkmap = $true
    $markmapCmd = "markmap"
    if (-not $isCargoBuild) { Write-Host "Using globally installed 'markmap' (fast)" -ForegroundColor Green }
}
# 2. Check for 'markmap-cli'
elseif (Get-Command markmap-cli -ErrorAction SilentlyContinue) {
    $useMarkmap = $true
    $markmapCmd = "markmap-cli"
    if (-not $isCargoBuild) { Write-Host "Using globally installed 'markmap-cli'" -ForegroundColor Green }
}
else {
    if (-not $isCargoBuild) {
        Write-Host "No global markmap found — will use npx (slower first run)" -ForegroundColor Yellow
    }
}

$success = 0
$failed = 0

foreach ($mmd in $mmdFiles) {
    $html = [System.IO.Path]::ChangeExtension($mmd.FullName, "html")
    Write-Host "-> $($mmd.Name)" -NoNewline -ForegroundColor White

    if ($useMarkmap) {
        & $markmapCmd --no-open $mmd.FullName -o $html | Out-Null
    } else {
        & cmd /c npx --yes markmap-cli --no-open $mmd.FullName -o $html | Out-Null
    }

    if ($LASTEXITCODE -eq 0) {
        Write-Host "  OK" -ForegroundColor Green
        $success = $success + 1
    } else {
        Write-Host "  FAIL" -ForegroundColor Red
        if ($useMarkmap) {
            Write-Host "   Manual: $markmapCmd $($mmd.FullName) -o $html" -ForegroundColor DarkGray
        } else {
            Write-Host "   Manual: npx markmap-cli $($mmd.FullName) -o $html" -ForegroundColor DarkGray
        }
        $failed = $failed + 1
    }
}

Write-Host ""
Write-Host ("Done: {0} succeeded, {1} failed" -f $success, $failed) -ForegroundColor Cyan