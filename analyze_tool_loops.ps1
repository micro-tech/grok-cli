# Analyze Tool Loops - Debug Script for ACP
# This script analyzes the acp_debug.log file to identify tool loop patterns

param(
    [string]$LogFile = "acp_debug.log",
    [int]$ShowLastN = 100
)

Write-Host "=== Grok CLI - Tool Loop Analyzer ===" -ForegroundColor Cyan
Write-Host ""

if (-not (Test-Path $LogFile))
{
    Write-Host "Error: Log file '$LogFile' not found!" -ForegroundColor Red
    Write-Host "Expected location: $(Join-Path $PWD $LogFile)" -ForegroundColor Yellow
    exit 1
}

Write-Host "Analyzing: $LogFile" -ForegroundColor Green
Write-Host "Reading last $ShowLastN lines..." -ForegroundColor Gray
Write-Host ""

# Read the log file
$lines = Get-Content $LogFile -Tail $ShowLastN

# Extract tool loop iterations
$toolLoops = $lines | Select-String "Tool loop iteration (\d+)/(\d+)" | ForEach-Object {
    if ($_ -match "Tool loop iteration (\d+)/(\d+)")
    {
        [PSCustomObject]@{
            Iteration = [int]$matches[1]
            MaxLoops  = [int]$matches[2]
            Line      = $_.Line
        }
    }
}

if ($toolLoops.Count -eq 0)
{
    Write-Host "No tool loop iterations found in the log." -ForegroundColor Yellow
    Write-Host "The log might be empty or from a different session." -ForegroundColor Gray
    exit 0
}

Write-Host "=== Tool Loop Summary ===" -ForegroundColor Cyan
Write-Host "Total iterations found: $($toolLoops.Count)" -ForegroundColor White
Write-Host "Max configured loops: $($toolLoops[0].MaxLoops)" -ForegroundColor White
Write-Host "Highest iteration reached: $($toolLoops[-1].Iteration)" -ForegroundColor White
Write-Host ""

# Check if max was reached
if ($toolLoops[-1].Iteration -ge $toolLoops[-1].MaxLoops)
{
    Write-Host "⚠️  WARNING: Max tool loop iterations was reached!" -ForegroundColor Red
    Write-Host "   The AI hit the safety limit and was stopped." -ForegroundColor Yellow
    Write-Host ""
}

# Extract tool calls
Write-Host "=== Tool Calls Made ===" -ForegroundColor Cyan
$toolCalls = $lines | Select-String "Tool \d+/\d+: (.+)" | ForEach-Object {
    if ($_ -match "Tool \d+/\d+: (.+)")
    {
        $matches[1]
    }
}

if ($toolCalls.Count -gt 0)
{
    $toolGroups = $toolCalls | Group-Object | Sort-Object Count -Descending

    foreach ($group in $toolGroups)
    {
        $percentage = [math]::Round(($group.Count / $toolCalls.Count) * 100, 1)
        Write-Host "  $($group.Name): $($group.Count) calls ($percentage%)" -ForegroundColor White
    }
    Write-Host ""

    # Check for suspicious patterns
    if ($toolGroups[0].Count -gt ($toolCalls.Count * 0.7))
    {
        Write-Host "⚠️  PATTERN DETECTED: '$($toolGroups[0].Name)' called repeatedly!" -ForegroundColor Red
        Write-Host "   This tool was called $($toolGroups[0].Count) times ($(([math]::Round(($toolGroups[0].Count / $toolCalls.Count) * 100, 1)))% of all calls)" -ForegroundColor Yellow
        Write-Host "   This suggests the AI is stuck in a loop with this tool." -ForegroundColor Yellow
        Write-Host ""
    }
}

# Extract finish reasons
Write-Host "=== Finish Reasons ===" -ForegroundColor Cyan
$finishReasons = $lines | Select-String "Finish reason: (.+)" | ForEach-Object {
    if ($_ -match 'Finish reason: (.+)')
    {
        $matches[1]
    }
}

if ($finishReasons.Count -gt 0)
{
    $reasonGroups = $finishReasons | Group-Object | Sort-Object Count -Descending

    foreach ($group in $reasonGroups)
    {
        Write-Host "  $($group.Name): $($group.Count) times" -ForegroundColor White
    }
    Write-Host ""

    # Check for tool_calls finish reason
    $toolCallsFinish = ($finishReasons | Where-Object { $_ -match "tool_calls" }).Count
    if ($toolCallsFinish -eq $finishReasons.Count)
    {
        Write-Host "⚠️  ISSUE: All responses finished with 'tool_calls' reason!" -ForegroundColor Red
        Write-Host "   The AI never signaled completion (should end with 'stop')." -ForegroundColor Yellow
        Write-Host "   This means the AI kept requesting more tool calls." -ForegroundColor Yellow
        Write-Host ""
    }
}

# Extract errors
Write-Host "=== Errors Found ===" -ForegroundColor Cyan
$errors = $lines | Select-String "ERROR|❌"

if ($errors.Count -gt 0)
{
    Write-Host "  Found $($errors.Count) errors:" -ForegroundColor Red
    foreach ($error in $errors | Select-Object -First 5)
    {
        Write-Host "    - $($error.Line)" -ForegroundColor Gray
    }
    if ($errors.Count -gt 5)
    {
        Write-Host "    ... and $($errors.Count - 5) more errors" -ForegroundColor Gray
    }
    Write-Host ""
} else
{
    Write-Host "  No errors found ✓" -ForegroundColor Green
    Write-Host ""
}

# Recommendations
Write-Host "=== Recommendations ===" -ForegroundColor Cyan

if ($toolLoops[-1].Iteration -ge $toolLoops[-1].MaxLoops)
{
    Write-Host "1. The AI hit the loop limit. This is usually NOT a limit problem." -ForegroundColor Yellow
    Write-Host "   Instead, the AI is stuck repeating the same action." -ForegroundColor Yellow
    Write-Host ""
}

if ($toolCalls.Count -gt 0 -and $toolGroups[0].Count -gt 5)
{
    Write-Host "2. Try being more specific in your prompt:" -ForegroundColor Yellow
    Write-Host "   - Instead of 'read the file', say 'read the file and tell me what's in it'" -ForegroundColor Gray
    Write-Host "   - Add 'then stop' or 'that's all I need' to your request" -ForegroundColor Gray
    Write-Host ""
}

if ($finishReasons.Count -gt 0)
{
    $toolCallsFinish = ($finishReasons | Where-Object { $_ -match "tool_calls" }).Count
    if ($toolCallsFinish -gt ($finishReasons.Count * 0.8))
    {
        Write-Host "3. The AI is not recognizing task completion:" -ForegroundColor Yellow
        Write-Host "   - This might be a prompt engineering issue" -ForegroundColor Gray
        Write-Host "   - Try rephrasing your request more explicitly" -ForegroundColor Gray
        Write-Host "   - Consider filing a bug report with your prompt and this log" -ForegroundColor Gray
        Write-Host ""
    }
}

Write-Host "4. For immediate debugging:" -ForegroundColor Yellow
Write-Host "   - Set a lower limit temporarily: " -ForegroundColor Gray -NoNewline
Write-Host "`$env:GROK_ACP_MAX_TOOL_LOOP_ITERATIONS = '10'" -ForegroundColor White
Write-Host "   - Check the full log: " -ForegroundColor Gray -NoNewline
Write-Host "Get-Content $LogFile | Select-String 'Tool|ERROR'" -ForegroundColor White
Write-Host ""

Write-Host "=== Analysis Complete ===" -ForegroundColor Cyan
Write-Host ""
Write-Host "TIP: Run this script after each session to track patterns:" -ForegroundColor Gray
Write-Host "     .\analyze_tool_loops.ps1 -ShowLastN 200" -ForegroundColor White
