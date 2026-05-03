# patch_appdata_config.ps1
# Adds max_context_tokens and max_tool_result_chars to the AppData config.toml
# Run with:  powershell -ExecutionPolicy Bypass -File scripts\patch_appdata_config.ps1

$src = "$env:APPDATA\grok-cli\config.toml"

if (-not (Test-Path $src))
{
    Write-Error "Config not found at $src"
    exit 1
}

$content = [System.IO.File]::ReadAllText($src)

$oldText = 'max_history_messages = 80'
$newText  = @"
max_history_messages = 80
# Soft token budget for the full outgoing request (history + system prompt +
# tool definitions). Messages are trimmed oldest-first when this is exceeded.
# Grok-3 / grok-beta context window = 256,000 tokens; 220,000 leaves ~36 k
# headroom for the model response and tool schemas.
# Default: 220000
max_context_tokens = 220000

# Maximum characters kept per tool-result message before truncation.
# Large file reads are the most common cause of context-window overflow.
# Set to 0 to disable per-message truncation.
# Default: 30000  (~7,500 tokens)
max_tool_result_chars = 30000
"@

if ($content -notlike "*max_context_tokens*")
{
    $content = $content.Replace($oldText, $newText)
    $tmp = [System.IO.Path]::GetTempFileName()
    [System.IO.File]::WriteAllText($tmp, $content, [System.Text.Encoding]::UTF8)
    Copy-Item $tmp $src -Force
    Remove-Item $tmp
    Write-Host "Patched: $src"
} else
{
    Write-Host "Already patched (max_context_tokens already present) - no changes made."
}
