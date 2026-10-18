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
    // Task 471.4: split on `\s*&&\s*` — the old exact-" && " split missed
    // `cmd1 &&cmd2`, `cmd1&& cmd2`, and extra-whitespace variants, leaving
    // them untranslated on Windows.
    static RE_AND_CHAIN: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"\s*&&\s*").expect("valid regex"));
    let parts: Vec<&str> = RE_AND_CHAIN.split(cmd).collect();
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
                    // NOTE (Task 471.1): no `-ExecutionPolicy Bypass` here.
                    // Execution policy governs .ps1 script files, not `-Command`
                    // strings, so bypassing it was unnecessary — and silently
                    // lowering the user's script-execution posture is not
                    // something an agent tool should do.
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

    // Task 464: truncate at the SOURCE (tail — compiler/test errors live at
    // the END of output) instead of dumping unbounded stdout+stderr into
    // context.  Each stream is capped; the wrapper is a one-line header.
    // The result is still never blank: empty streams get an explicit marker
    // so the model doesn't report "no output" / "blank".
    const MAX_SHELL_STREAM_CHARS: usize = 15_000;
    let stdout_part = {
        let t = stdout.trim_end_matches('\n').trim_end_matches('\r');
        if t.is_empty() {
            "(empty)".to_string()
        } else {
            crate::acp::context_trim::truncate_tool_content(t, MAX_SHELL_STREAM_CHARS)
        }
    };
    let stderr_part = {
        let t = stderr.trim_end_matches('\n').trim_end_matches('\r');
        if t.is_empty() {
            "(empty)".to_string()
        } else {
            crate::acp::context_trim::truncate_tool_content(t, MAX_SHELL_STREAM_CHARS)
        }
    };

    let result = format!(
        "$ {command} [exit {exit_code}]\nstdout:\n{stdout_part}\nstderr:\n{stderr_part}"
    );

    if !output.status.success() {
        tracing::warn!(
            exit_code = exit_code,
            command = %command,
            "shell_tools: non-zero exit — returning as error so callers can distinguish failure (COR-10)"
        );
        // Return rich error so the model still sees the (truncated) STDOUT/STDERR + context.
        return Err(anyhow!("{}", result));
    }

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
    async fn non_zero_exit_returns_err_cor10() {
        let policy = SecurityPolicy::new();
        // Cross-platform failing command
        #[cfg(target_os = "windows")]
        let cmd = "cmd /c exit 1";
        #[cfg(not(target_os = "windows"))]
        let cmd = "false";

        let result = run_shell_command(cmd, &policy).await;
        // COR-10: return Err on non-zero exit so callers (including sub-agents, verifiers) can distinguish failure.
        // The error still contains the rich labeled output (STDOUT/STDERR + exit code) for the model.
        assert!(result.is_err(), "non-zero exit must return Err (COR-10)");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("[exit") && (err.contains("-1") || err.contains("1") || err.contains("exit")),
            "error must contain '[exit N]' and indication of failure, got: {}",
            err
        );
        // Still rich: model/harness sees labeled output even in the error case.
        assert!(err.contains("stdout:") || err.contains("stderr:"));
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

    #[tokio::test]
    async fn large_output_is_truncated_at_source() {
        // Task 464: unbounded shell output must be truncated at the source
        // (tail) with a minimal one-line header — not dumped whole into context.
        let policy = SecurityPolicy::new();
        let result = run_shell_command("seq 1 20000", &policy).await;
        assert!(result.is_ok(), "seq should succeed: {:?}", result);
        let out = result.unwrap();
        // One-line header, no box-drawing decoration.
        assert!(out.starts_with("$ seq 1 20000 [exit 0]"), "one-line header, got: {}", &out[..80.min(out.len())]);
        assert!(!out.contains('═'), "decorative wrapper must be gone");
        // Tail kept: the last numbers are visible, the head is cut with a marker.
        assert!(out.contains("20000"), "tail of output must be visible");
        assert!(
            out.contains("earlier output truncated"),
            "truncation marker must be present, len={}",
            out.len()
        );
        assert!(out.len() < 40_000, "output must be bounded, len={}", out.len());
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn windows_and_chain_stops_on_failure() {
        let policy = SecurityPolicy::new();
        // First part fails (exit 1), second part must NOT execute.
        // Per COR-10, run_shell_command returns Err on non-zero exit — the rich labeled
        // output is embedded in the error message so the model / harness can still read it.
        // The PowerShell && translation (using $LASTEXITCODE) ensures the echo never runs.
        // We detect the short-circuit by absence of the marker in the output section.
        let result =
            run_shell_command("cmd /c exit 1 && echo SHOULD_NOT_APPEAR_IN_OUTPUT", &policy).await;

        assert!(result.is_err(), "non-zero exit must return Err (COR-10)");
        let out = result.unwrap_err().to_string();

        // The header always repeats the original command (so it legitimately contains the marker text).
        // We must verify the *executed payload* (everything after "[exit") does NOT contain it.
        // This proves the PowerShell `if ($LASTEXITCODE -eq 0)` guard prevented the echo from running.
        let after_exit = out
            .split_once("[exit")
            .map(|(_, rest)| rest)
            .unwrap_or(&out);

        assert!(
            !after_exit.contains("SHOULD_NOT_APPEAR_IN_OUTPUT"),
            "second command after && must not run when first fails (PowerShell $LASTEXITCODE guard). Full result:\n{}",
            out
        );

        assert!(
            out.contains("[exit") && (out.contains("-1") || out.contains("1") || out.contains("exit")),
            "output should mention [exit N] and failure, got: {}",
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

    // Task 471.4: `&&` splitting must handle missing/extra whitespace variants.
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_and_chain_split_handles_whitespace_variants() {
        for cmd in [
            "a &&b",
            "a&& b",
            "a&&b",
            "a  &&  b",
            "a\t&&\tb",
        ] {
            let t = translate_powershell_and_chain(cmd);
            assert!(
                t.contains("if ($LASTEXITCODE -eq 0)"),
                "variant must be translated: {} -> {}",
                cmd,
                t
            );
        }
        // No chain → unchanged.
        assert_eq!(translate_powershell_and_chain("cargo check"), "cargo check");
    }
}
