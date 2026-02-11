# Cleanup Old Grok Installation Script
# This script removes the old grok.exe from the Cargo bin directory
# to prevent version conflicts

Write-Host "Grok CLI - Cleanup Old Installation" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan
Write-Host ""

# Check if we're on Windows
if (-not $IsWindows -and $PSVersionTable.PSVersion.Major -lt 6)
{
    # PowerShell 5.1 on Windows doesn't have $IsWindows variable
    $IsWindows = $true
}

if (-not $IsWindows)
{
    Write-Host "This script is designed for Windows only." -ForegroundColor Red
    exit 1
}

# Find Cargo bin directory
$cargoBin = "$env:USERPROFILE\.cargo\bin\grok.exe"
$localAppData = "$env:LOCALAPPDATA\grok-cli\bin\grok.exe"

Write-Host "Checking for installed versions..." -ForegroundColor Yellow
Write-Host ""

# Check Cargo version
if (Test-Path $cargoBin)
{
    Write-Host "[FOUND] Old Cargo installation:" -ForegroundColor Yellow
    Write-Host "  Location: $cargoBin" -ForegroundColor Gray

    # Try to get version
    try
    {
        $cargoVersion = & $cargoBin --version 2>$null
        Write-Host "  Version:  $cargoVersion" -ForegroundColor Gray
    } catch
    {
        Write-Host "  Version:  Unable to determine" -ForegroundColor Gray
    }

    $removeOld = $true
} else
{
    Write-Host "[OK] No old Cargo installation found." -ForegroundColor Green
    $removeOld = $false
}

Write-Host ""

# Check new version
if (Test-Path $localAppData)
{
    Write-Host "[FOUND] New installation:" -ForegroundColor Green
    Write-Host "  Location: $localAppData" -ForegroundColor Gray

    # Try to get version
    try
    {
        $newVersion = & $localAppData --version 2>$null
        Write-Host "  Version:  $newVersion" -ForegroundColor Gray
    } catch
    {
        Write-Host "  Version:  Unable to determine" -ForegroundColor Gray
    }
} else
{
    Write-Host "[WARNING] New installation not found at: $localAppData" -ForegroundColor Yellow
    Write-Host "          Please run the installer first!" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Remove old installation if found
if ($removeOld)
{
    Write-Host "Do you want to remove the old Cargo installation?" -ForegroundColor Yellow
    Write-Host "This will delete: $cargoBin" -ForegroundColor Gray
    Write-Host ""

    $confirmation = Read-Host "Type 'yes' to continue or 'no' to cancel"

    if ($confirmation -eq "yes")
    {
        Write-Host ""
        Write-Host "Removing old installation..." -ForegroundColor Cyan

        try
        {
            Remove-Item $cargoBin -Force -ErrorAction Stop
            Write-Host "[SUCCESS] Old Cargo installation removed!" -ForegroundColor Green
            Write-Host ""
            Write-Host "You may need to restart your PowerShell session or run:" -ForegroundColor Yellow
            Write-Host "  refreshenv" -ForegroundColor Gray
            Write-Host "or close and reopen PowerShell to use the new version." -ForegroundColor Yellow
        } catch
        {
            Write-Host "[ERROR] Failed to remove old installation: $_" -ForegroundColor Red
            Write-Host ""
            Write-Host "The file may be in use. Please:" -ForegroundColor Yellow
            Write-Host "  1. Close all running grok instances" -ForegroundColor Gray
            Write-Host "  2. Close all PowerShell/terminal windows using grok" -ForegroundColor Gray
            Write-Host "  3. Run this script again" -ForegroundColor Gray
            exit 1
        }
    } else
    {
        Write-Host ""
        Write-Host "[CANCELLED] Old installation was not removed." -ForegroundColor Yellow
        Write-Host "You can manually delete it later if needed." -ForegroundColor Gray
    }
} else
{
    Write-Host "[OK] No cleanup needed!" -ForegroundColor Green
}

Write-Host ""
Write-Host "Cleanup complete!" -ForegroundColor Green
Write-Host ""

# Verify which grok will be used
Write-Host "Verifying grok command..." -ForegroundColor Cyan
try
{
    $grokPath = (Get-Command grok -ErrorAction Stop).Path
    Write-Host "  Current grok path: $grokPath" -ForegroundColor Gray

    $currentVersion = & grok --version 2>$null | Select-Object -First 1
    Write-Host "  Current version:   $currentVersion" -ForegroundColor Gray

    # Check if it's the right one
    if ($grokPath -like "*AppData\Local\grok-cli*")
    {
        Write-Host ""
        Write-Host "[SUCCESS] You are using the correct version!" -ForegroundColor Green
    } elseif ($grokPath -like "*\.cargo\bin*")
    {
        Write-Host ""
        Write-Host "[WARNING] Still using Cargo version!" -ForegroundColor Yellow
        Write-Host "          Please restart PowerShell or run: refreshenv" -ForegroundColor Yellow
    } else
    {
        Write-Host ""
        Write-Host "[INFO] Using grok from: $grokPath" -ForegroundColor Cyan
    }
} catch
{
    Write-Host "  [ERROR] 'grok' command not found in PATH" -ForegroundColor Red
    Write-Host "  Please make sure the installer completed successfully." -ForegroundColor Yellow
}

Write-Host ""
