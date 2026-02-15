# Fix config.toml syntax errors
# This script fixes common TOML syntax errors in the grok-cli config file

$configPath = "$env:APPDATA\grok-cli\config.toml"

Write-Host "Fixing config.toml syntax errors..." -ForegroundColor Cyan
Write-Host "Config path: $configPath" -ForegroundColor Yellow

if (-not (Test-Path $configPath))
{
    Write-Host "Config file not found at: $configPath" -ForegroundColor Red
    Write-Host "Run 'grok config init' to create a config file first." -ForegroundColor Yellow
    exit 1
}

# Backup the original file
$backupPath = "$configPath.backup"
Copy-Item $configPath $backupPath -Force
Write-Host "Created backup: $backupPath" -ForegroundColor Green

# Read the config file
$content = Get-Content $configPath -Raw

# Fix 1: Remove commas from numbers (TOML doesn't support number formatting)
# Examples: 256,000 -> 256000, 1,000 -> 1000
$originalContent = $content
$content = $content -replace '(\d+),(\d{3})', '$1$2'

# Check if any changes were made
if ($content -ne $originalContent)
{
    Write-Host "Fixed number formatting (removed commas)" -ForegroundColor Green
}

# Save the fixed content
Set-Content $configPath $content -NoNewline

Write-Host "`nConfig file fixed successfully!" -ForegroundColor Green
Write-Host "Original backed up to: $backupPath" -ForegroundColor Cyan

# Verify the config is valid
Write-Host "`nVerifying config..." -ForegroundColor Cyan
$verification = & grok config show 2>&1
if ($LASTEXITCODE -eq 0)
{
    Write-Host "✓ Configuration is valid!" -ForegroundColor Green
} else
{
    Write-Host "⚠ There may still be issues with the config:" -ForegroundColor Yellow
    Write-Host $verification -ForegroundColor Red
    Write-Host "`nYou can restore the backup with:" -ForegroundColor Yellow
    $restoreCmd = "Copy-Item '$backupPath' '$configPath' -Force"
    Write-Host "  $restoreCmd" -ForegroundColor White
}
