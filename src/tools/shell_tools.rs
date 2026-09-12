//! Shell tool — executes a single command in the working directory.

use crate::acp::security::SecurityPolicy;
use anyhow::{Result, anyhow};
use tokio::process::Command;
use tokio::time::{Duration, timeout};
use tracing::warn;

/// Return the effective shell-command timeout in seconds.
///
/// Priority (highest → lowest):
/// 1. `GROK_SHELL_TIMEOUT_SECS` environment variable — one-off override
///    without touching config files.
/// 2. `tools.shell.command_timeout_secs` in `config.toml` — loaded into
///    the [`SecurityPolicy`] at startup by `GrokAcpAgent::new`.
/// 3. 300 s compiled-in safety net (used only if neither of the above is set).
fn effective_timeout(security: &SecurityPolicy) -> u64 {
    std::env::var("GROK_SHELL_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&t| t > 0)
        .unwrap_or_else(|| security.shell_timeout_secs())
}

/// Translate a bash-style `&&` chain into PowerShell that respects
/// "run next only on success".
///
/// Example:
///   "cargo check && cargo test"
/// becomes (roughly):
///   "cargo check; if ($LASTEXITCODE -eq 0) { cargo test }"
///
/// This is required because PowerShell `;` runs unconditionally,
/// while bash `&&` short-circuits on failure.
#[cfg(target_os = "windows")]
fn translate_powershell_and_chain(cmd: &str) -> String {
    // Split on the exact " && " sequence the original code used.
    // This keeps the translation simple and predictable.
    let parts: Vec<&str> = cmd.split(" && ").collect();
    if parts.len() <= 1 {
        return cmd.to_string();
    }

    let mut result = String::new();
    for (i, part) in parts.iter().enumerate() {
        let trimmed = part.trim();
        if i == 0 {
            result.push_str(trimmed);
        } else {
            // After previous command, only run this one if exit code was 0.
            result.push_str(&format!("; if ($LASTEXITCODE -eq 0) {{ {} }}", trimmed));
        }
    }
    result
}

/// Run a shell command with a hard execution timeout.
///
/// # Security
/// - [`SecurityPolicy::validate_shell_command`] is called first to check the
///   denylist — the command is rejected before any subprocess is spawned.
/// - The command runs in the session's working directory so it cannot
///   accidentally affect files outside the project root.
/// - On **Windows**, PowerShell is invoked with `-NonInteractive -NoProfile
///   -ExecutionPolicy Bypass`.
///   Bash-style `&&` (run-next-only-on-success) is correctly translated so
///   later commands do **not** run if an earlier one fails. We use a
///   `$LASTEXITCODE` conditional that works on both PowerShell 5.1 and 7+.
/// - Execution is bounded by [`effective_timeout`]; if the process does not
///   finish in time an error is returned (the child is killed by the OS when
///   the `Command` future is dropped).
///
/// # Errors
/// Returns an error if the command is on the denylist, fails to spawn, or
/// exceeds the timeout.
///
/// The timeout is determined in priority order by: the `GROK_SHELL_TIMEOUT_SECS`
/// environment variable, `tools.shell.command_timeout_secs` in `config.toml`,
/// or a 300 s compiled-in fallback.
pub async fn run_shell_command(command: &str, security: &SecurityPolicy) -> Result<String> {
    security.validate_shell_command(command)?;

    let cwd = security.working_directory().to_path_buf();
    let timeout_secs = effective_timeout(security);
    let timeout_duration = Duration::from_secs(timeout_secs);

    // Compile-time platform branching so that `translate_powershell_and_chain`
    // (which only exists on Windows) is never referenced on other platforms.
    let spawn_result = {
        #[cfg(target_os = "windows")]
        {
            // Bash-style `&&` means "run next command only if previous succeeded".
            // PowerShell `;` is unconditional (like `; ` in bash).
            // We translate `&&` chains into conditional blocks using $LASTEXITCODE.
            // This works on both Windows PowerShell 5.1 and PowerShell 7+ (pwsh).
            let ps_command = translate_powershell_and_chain(command);

            Command::new("powershell")
                .args([
                    "-NonInteractive",
                    "-NoProfile",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    &ps_command,
                ])
                .current_dir(&cwd)
                .output()
        }
        #[cfg(not(target_os = "windows"))]
        {
            Command::new("sh")
                .args(["-c", command])
                .current_dir(&cwd)
                .output()
        }
    };

    // Wrap execution in a hard timeout.
    let output = match timeout(timeout_duration, spawn_result).await {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => {
            tracing::warn!(
                command = command,
                error = %e,
                "shell_tools: failed to spawn command"
            );
            return Err(anyhow!("Failed to spawn command: {}", e));
        }
        Err(_) => {
            warn!(
                command = %command,
                timeout_secs = timeout_secs,
                "Shell command timed out"
            );
            return Err(anyhow!(
                "Command timed out after {}s: {}",
                timeout_secs,
                command
            ));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    // HARNESS / ACP FIX: The LLM (especially inside HOH harness, sub-agents, and ACP chat)
    // frequently reports "tool call returned blank" or "no reply" for run_shell_command.
    // Root causes we are killing here:
    //   - Old code returned Err on non-zero → result became wrapped error without raw output.
    //   - Empty stdout+stderr looked like nothing.
    //   - Important compiler/test errors live at the END of output (head truncation hid them).
    //
    // Solution: ALWAYS return a big, labeled, never-blank string. Include command + exit + both streams
    // (or clear "(empty)" markers). This string goes straight into the "tool" role message the LLM sees.

    let stdout_clean = stdout.trim_end_matches('\n').trim_end_matches('\r');
    let stderr_clean = stderr.trim_end_matches('\n').trim_end_matches('\r');

    let body = if stdout_clean.is_empty() && stderr_clean.is_empty() {
        "OUTPUT: (no stdout and no stderr were produced by the command)".to_string()
    } else {
        format!(
            "STDOUT ({} bytes):\n{}\n\nSTDERR ({} bytes):\n{}",
            stdout_clean.len(),
            if stdout_clean.is_empty() { "(empty)" } else { stdout_clean },
            stderr_clean.len(),
            if stderr_clean.is_empty() { "(empty)" } else { stderr_clean }
        )
    };

    let result = format!(
        "═══════════════════════════════════════════════════════════════\n\
         TOOL RESULT: run_shell_command\n\
         Command: {}\n\
         Exit code: {}\n\
         {}\n\
         ═══════════════════════════════════════════════════════════════\n\
         (LLM: this is the COMPLETE output. Do not say \"no output\" or \"blank\".)",
        command,
        exit_code,
        body
    );

    if !output.status.success() {
        tracing::warn!(
            exit_code = exit_code,
            command = %command,
            "shell_tools: non-zero exit — rich output returned to LLM/harness anyway"
        );
    }

    // Critical: always Ok so the content reaches the model as a normal tool result.
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::security::SecurityPolicy;

    #[tokio::test]
    async fn echo_command_succeeds() {
        let policy = SecurityPolicy::new();
        let result = run_shell_command("echo hello", &policy).await;
        assert!(result.is_ok(), "echo should succeed: {:?}", result);
        let out = result.unwrap();
        assert!(
            out.contains("hello"),
            "output should contain 'hello': {}",
            out
        );
    }

    #[tokio::test]
    async fn non_zero_exit_still_returns_output() {
        let policy = SecurityPolicy::new();
        // Cross-platform failing command
        #[cfg(target_os = "windows")]
        let cmd = "cmd /c exit 1";
        #[cfg(not(target_os = "windows"))]
        let cmd = "false";

        let result = run_shell_command(cmd, &policy).await;
        // Changed behavior (COR-10 + ACP visibility): we now return Ok with the output
        // so the model always sees the real stdout/stderr even when the command "fails".
        assert!(result.is_ok(), "non-zero exit must still return Ok so output is visible (ACP/tool result)");
        let out = result.unwrap();
        assert!(
            out.contains("Exit code:") && (out.contains("-1") || out.contains("1") || out.contains("exit")),
            "output must contain 'Exit code:' and indication of failure, got: {}",
            out
        );
    }

    #[tokio::test]
    async fn blocked_command_is_rejected() {
        let policy = SecurityPolicy::new();
        // "rm -rf" is on the denylist; must be rejected before spawning.
        let result = run_shell_command("rm -rf /tmp/should_not_exist", &policy).await;
        assert!(result.is_err(), "dangerous command should be blocked");
    }

    // ── PowerShell && chaining tests (Windows only) ───────────────────────────
    //
    // These verify that `&&` is translated into a conditional that respects
    // "run next command only if the previous one succeeded".
    // The naive `replace(" && ", "; ")` would have let the second command run
    // unconditionally.

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn windows_and_chain_stops_on_failure() {
        let policy = SecurityPolicy::new();
        // First part fails (exit 1), second part must NOT execute.
        // run_shell_command ALWAYS returns Ok (rich labeled output for harness/ACP/LLM).
        // The PowerShell && translation (using $LASTEXITCODE) ensures the echo never runs.
        // We detect the short-circuit by absence of the marker + presence of failure exit code.
        let result =
            run_shell_command("cmd /c exit 1 && echo SHOULD_NOT_APPEAR_IN_OUTPUT", &policy).await;

        assert!(result.is_ok(), "non-zero exit must return Ok (rich output for harness/ACP)");
        let out = result.unwrap();

        // The header always repeats the original command (so it legitimately contains the marker text).
        // We must verify the *executed payload* (everything after "Exit code:") does NOT contain it.
        // This proves the PowerShell `if ($LASTEXITCODE -eq 0)` guard prevented the echo from running.
        let after_exit = out
            .split_once("Exit code:")
            .map(|(_, rest)| rest)
            .unwrap_or(&out);

        assert!(
            !after_exit.contains("SHOULD_NOT_APPEAR_IN_OUTPUT"),
            "second command after && must not run when first fails (PowerShell $LASTEXITCODE guard). Full result:\n{}",
            out
        );

        assert!(
            out.contains("Exit code:") && (out.contains("-1") || out.contains("1") || out.contains("exit")),
            "output should mention Exit code and failure, got: {}",
            out
        );
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn windows_and_chain_runs_second_on_success() {
        let policy = SecurityPolicy::new();
        let result = run_shell_command("cmd /c exit 0 && echo CHAIN_SUCCESS_MARKER", &policy).await;

        assert!(result.is_ok(), "successful chain should succeed");
        let out = result.unwrap();
        assert!(
            out.contains("CHAIN_SUCCESS_MARKER"),
            "second command should have run. Got: {}",
            out
        );
    }
}
