# Test ACP with debug logging
Write-Host "Starting ACP test..." -ForegroundColor Cyan
Write-Host "Log file will be: acp_test.log" -ForegroundColor Yellow

$env:RUST_LOG="debug,grok_cli::acp=trace"
.\target\release\grok.exe acp --stdio 2>&1 | Tee-Object -FilePath acp_test.log
