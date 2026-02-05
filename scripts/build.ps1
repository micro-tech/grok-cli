#!/usr/bin/env pwsh
# PowerShell Build Script for grok-cli
#
# This script builds the project and handles common build tasks
# Can be used locally or in CI/CD pipelines (GitHub Actions)

param(
    [switch]$Release,
    [switch]$Clean,
    [switch]$Test,
    [switch]$Clippy,
    [switch]$Doc,
    [switch]$All,
    [switch]$Verbose,
    [string]$Target = ""
)

# Colors for output
function Write-Success
{ param($msg) Write-Host "✓ $msg" -ForegroundColor Green 
}
function Write-Error-Custom
{ param($msg) Write-Host "✗ $msg" -ForegroundColor Red 
}
function Write-Info
{ param($msg) Write-Host "ℹ $msg" -ForegroundColor Cyan 
}
function Write-Step
{ param($msg) Write-Host "→ $msg" -ForegroundColor Yellow 
}

# Get script directory
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir

# Change to project root
Set-Location $ProjectRoot

Write-Info "=== Grok-CLI Build Script ==="
Write-Info "Project: $ProjectRoot"
Write-Info ""

# Build flags
$BuildFlags = @()
if ($Release)
{
    $BuildFlags += "--release"
    Write-Info "Build Mode: Release"
} else
{
    Write-Info "Build Mode: Debug"
}

if ($Verbose)
{
    $BuildFlags += "--verbose"
}

if ($Target)
{
    $BuildFlags += "--target", $Target
    Write-Info "Target: $Target"
}

# Track success
$Success = $true

# Clean
if ($Clean -or $All)
{
    Write-Step "Cleaning build artifacts..."
    cargo clean
    if ($LASTEXITCODE -eq 0)
    {
        Write-Success "Clean complete"
    } else
    {
        Write-Error-Custom "Clean failed"
        $Success = $false
    }
    Write-Host ""
}

# Format check
if ($All)
{
    Write-Step "Checking code formatting..."
    cargo fmt -- --check
    if ($LASTEXITCODE -eq 0)
    {
        Write-Success "Format check passed"
    } else
    {
        Write-Error-Custom "Format check failed - run 'cargo fmt' to fix"
        $Success = $false
    }
    Write-Host ""
}

# Clippy (lints)
if ($Clippy -or $All)
{
    Write-Step "Running Clippy linter..."
    $ClippyArgs = @("clippy")
    if ($BuildFlags.Count -gt 0)
    {
        $ClippyArgs += $BuildFlags
    }
    $ClippyArgs += "--", "-D", "warnings"

    & cargo @ClippyArgs
    if ($LASTEXITCODE -eq 0)
    {
        Write-Success "Clippy passed"
    } else
    {
        Write-Error-Custom "Clippy found issues"
        $Success = $false
    }
    Write-Host ""
}

# Build
Write-Step "Building project..."
$BuildArgs = @("build")
if ($BuildFlags.Count -gt 0)
{
    $BuildArgs += $BuildFlags
}

& cargo @BuildArgs
if ($LASTEXITCODE -eq 0)
{
    Write-Success "Build complete"
} else
{
    Write-Error-Custom "Build failed"
    $Success = $false
    exit 1
}
Write-Host ""

# Test
if ($Test -or $All)
{
    Write-Step "Running tests..."
    $TestArgs = @("test")
    if ($BuildFlags.Count -gt 0)
    {
        $TestArgs += $BuildFlags
    }

    & cargo @TestArgs
    if ($LASTEXITCODE -eq 0)
    {
        Write-Success "Tests passed"
    } else
    {
        Write-Error-Custom "Tests failed"
        $Success = $false
    }
    Write-Host ""
}

# Documentation
if ($Doc -or $All)
{
    Write-Step "Building documentation..."
    cargo doc --no-deps
    if ($LASTEXITCODE -eq 0)
    {
        Write-Success "Documentation built"
    } else
    {
        Write-Error-Custom "Documentation build failed"
        $Success = $false
    }
    Write-Host ""
}

# Binary information
if ($Success)
{
    Write-Info "=== Build Summary ==="

    $BinPath = if ($Release)
    { "target/release" 
    } else
    { "target/debug" 
    }

    if ($IsWindows -or $env:OS -eq "Windows_NT")
    {
        $BinaryPath = Join-Path $BinPath "grok.exe"
    } else
    {
        $BinaryPath = Join-Path $BinPath "grok"
    }

    if (Test-Path $BinaryPath)
    {
        $BinarySize = (Get-Item $BinaryPath).Length
        $SizeMB = [math]::Round($BinarySize / 1MB, 2)
        Write-Info "Binary: $BinaryPath"
        Write-Info "Size: $SizeMB MB"

        # Try to get version
        try
        {
            $Version = & $BinaryPath --version 2>$null
            if ($LASTEXITCODE -eq 0)
            {
                Write-Info "Version: $Version"
            }
        } catch
        {
            # Ignore if binary can't run
        }
    }

    Write-Host ""
    Write-Success "Build completed successfully!"
    exit 0
} else
{
    Write-Host ""
    Write-Error-Custom "Build completed with errors"
    exit 1
}
