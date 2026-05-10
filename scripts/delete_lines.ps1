# Delete old handle_session_prompt, send_available_commands_update, send_text_update
# These are all dead code replaced by the new Builder-based implementation.
$f = 'src\cli\commands\acp.rs'
$lines = Get-Content $f
# Keep lines 0..1436 (before old handle_session_prompt) and 1959.. (from test_acp_connection)
$keep = $lines[0..1436] + $lines[1959..($lines.Length - 1)]
Set-Content $f $keep
Write-Host "Done. New line count: $($keep.Length)"
