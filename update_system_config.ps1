# Update System Config - Add MCP Server Section
# This script safely adds the MCP server configuration section to your system config

$configPath = "$env:APPDATA\grok-cli\config.toml"
$backupPath = "$env:APPDATA\grok-cli\config.toml.backup.$(Get-Date -Format 'yyyyMMdd_HHmmss')"

Write-Host "=== Grok CLI - System Config Updater ===" -ForegroundColor Cyan
Write-Host ""

# Check if config exists
if (-not (Test-Path $configPath))
{
    Write-Host "Error: Config file not found at: $configPath" -ForegroundColor Red
    Write-Host "Expected location: $configPath" -ForegroundColor Yellow
    exit 1
}

Write-Host "Found config file: $configPath" -ForegroundColor Green
Write-Host ""

# Create backup
Write-Host "Creating backup..." -ForegroundColor Yellow
try
{
    Copy-Item $configPath $backupPath -Force
    Write-Host "✓ Backup created: $backupPath" -ForegroundColor Green
} catch
{
    Write-Host "Error creating backup: $_" -ForegroundColor Red
    exit 1
}

Write-Host ""

# Check if MCP section already exists
$content = Get-Content $configPath -Raw
if ($content -match '\[mcp\.servers\.')
{
    Write-Host "MCP server section already exists in config file." -ForegroundColor Yellow
    Write-Host "No changes needed." -ForegroundColor Green
    Write-Host ""
    Write-Host "If you need to modify MCP servers, edit: $configPath" -ForegroundColor Gray
    exit 0
}

# Add MCP section
Write-Host "Adding MCP server configuration section..." -ForegroundColor Yellow

$mcpSection = @"

# MCP (Model Context Protocol) Servers Configuration
# Configure external MCP servers for extended functionality
# Uncomment and customize the examples below to enable MCP servers

# Example: GitHub MCP Server
# [mcp.servers.github]
# type = "stdio"
# command = "path/to/github_mcp.exe"
# args = []
# env = {}  # Required field - environment variables for the MCP server

# Example: Custom MCP Server with environment variables
# [mcp.servers.custom]
# type = "stdio"
# command = "/usr/local/bin/custom-mcp-server"
# args = ["--verbose", "--config", "/path/to/config"]
# env = { LOG_LEVEL = "debug", API_KEY = "your-key-here" }

# Note: The 'env' field is REQUIRED for each MCP server, even if empty ({})
# This prevents TOML parsing errors during configuration loading
"@

try
{
    Add-Content -Path $configPath -Value $mcpSection -NoNewline
    Write-Host "✓ MCP section added successfully" -ForegroundColor Green
} catch
{
    Write-Host "Error adding MCP section: $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Restoring backup..." -ForegroundColor Yellow
    Copy-Item $backupPath $configPath -Force
    Write-Host "✓ Backup restored" -ForegroundColor Green
    exit 1
}

Write-Host ""
Write-Host "=== Update Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "Your system config has been updated with MCP server templates." -ForegroundColor Green
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Edit the config file: $configPath" -ForegroundColor White
Write-Host "2. Uncomment and customize any MCP servers you want to use" -ForegroundColor White
Write-Host "3. Ensure each MCP server has an 'env' field (even if empty)" -ForegroundColor White
Write-Host ""
Write-Host "Backup saved at: $backupPath" -ForegroundColor Gray
Write-Host ""
Write-Host "To verify the config loads correctly, run:" -ForegroundColor Gray
Write-Host "  cargo run --bin grok -- config show" -ForegroundColor White
Write-Host ""
