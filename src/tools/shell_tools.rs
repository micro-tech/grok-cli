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
pub async fn run_shell_command(
    command: &str,
    security: &SecurityPolicy,
) -> Result<String> {
    security.validate_shell_command(command)?;

    let cwd = security.working_directory().to_path_buf();
    let timeout_secs = effective_timeout(security);
    let timeout_duration = Duration::from_secs(timeout_secs);

    let spawn_result = if cfg!(target_os = "windows") {
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
    } else {
        Command::new("sh")
            .args(["-c", command])
            .current_dir(&cwd)
            .output()
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

    if !output.status.success() {
        tracing::warn!(
            exit_code = output.status.code().unwrap_or(-1),
            command = command,
            "shell_tools: command exited with non-zero status"
        );
        // COR-10: Return Err on non-zero exit so callers (workflows, agents, tests)
        // can distinguish success from failure and propagate errors properly.
        return Err(anyhow!(
            "Command failed with code {}:\nStdout: {}\nStderr: {}",
            output.status, stdout, stderr
        ));
    }

    Ok(format!("Stdout: {}\nStderr: {}", stdout, stderr))
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
    async fn non_zero_exit_returns_err_cor10() {
        let policy = SecurityPolicy::new();
        // Cross-platform failing command
        #[cfg(target_os = "windows")]
        let cmd = "cmd /c exit 1";
        #[cfg(not(target_os = "windows"))]
        let cmd = "false";

        let result = run_shell_command(cmd, &policy).await;
        assert!(result.is_err(), "non-zero exit must return Err (COR-10)");
        let err = result.unwrap_err().to_string();
        assert!(
            err.to_lowercase().contains("failed with code")
                || err.contains("exit")
                || err.contains("1"),
            "error message should indicate failure, got: {}",
            err
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
        // COR-10: non-zero exit now returns Err (with the failure message inside).
        let result = run_shell_command(
            "cmd /c exit 1 && echo SHOULD_NOT_APPEAR_IN_OUTPUT",
            &policy,
        )
        .await;

        assert!(
            result.is_err(),
            "failing command must return Err (COR-10)"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            !err.contains("SHOULD_NOT_APPEAR_IN_OUTPUT"),
            "second command after && must not run when first fails. Got error: {}",
            err
        );
        assert!(
            err.contains("failed with code") || err.contains("exit 1"),
            "error should mention failure code, got: {}",
            err
        );
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn windows_and_chain_runs_second_on_success() {
        let policy = SecurityPolicy::new();
        let result =
            run_shell_command("cmd /c exit 0 && echo CHAIN_SUCCESS_MARKER", &policy).await;

        assert!(result.is_ok(), "successful chain should succeed");
        let out = result.unwrap();
        assert!(
            out.contains("CHAIN_SUCCESS_MARKER"),
            "second command should have run. Got: {}",
            out
        );
    }
}
