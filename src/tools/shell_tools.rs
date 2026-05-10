//! Shell tool — executes a single command in the working directory.

use crate::acp::security::SecurityPolicy;
use anyhow::{Result, anyhow};
use tokio::process::Command;
use tokio::time::{Duration, timeout};
use tracing::warn;

/// Default hard execution timeout for every shell command.
/// 300 s (5 min) is a safer default on Windows — `cargo build` and
/// `git status` on slow or Starlink-connected machines routinely exceed 30 s.
/// Override by passing an explicit value to [`run_shell_command`].
const DEFAULT_SHELL_TIMEOUT_SECS: u64 = 300;

/// Run a shell command with a hard execution timeout.
///
/// # Security
/// - [`SecurityPolicy::validate_shell_command`] is called first to check the
///   denylist — the command is rejected before any subprocess is spawned.
/// - The command runs in the session's working directory so it cannot
///   accidentally affect files outside the project root.
/// - On **Windows**, PowerShell is invoked with `-NonInteractive -NoProfile
///   -ExecutionPolicy Bypass` to prevent profile side-effects and hangs.
///   Bash-style `&&` chaining is rewritten to PowerShell `;`.
/// - Execution is bounded by `timeout_secs` (defaults to
///   [`DEFAULT_SHELL_TIMEOUT_SECS`] when 0 is passed); if the process does not
///   finish in time an error is returned (the child is killed by the OS when
///   the `Command` future is dropped).
///
/// # Errors
/// Returns an error if the command is on the denylist, fails to spawn, or
/// exceeds the timeout.
///
/// Pass `timeout_secs = 0` to use the built-in [`DEFAULT_SHELL_TIMEOUT_SECS`]
/// default (300 s).  Pass an explicit value to honour the project config
/// (`tools.shell.command_timeout_secs`).
pub async fn run_shell_command(
    command: &str,
    security: &SecurityPolicy,
    timeout_secs: u64,
) -> Result<String> {
    security.validate_shell_command(command)?;

    let cwd = security.working_directory().to_path_buf();
    let effective_timeout = if timeout_secs == 0 {
        DEFAULT_SHELL_TIMEOUT_SECS
    } else {
        timeout_secs
    };
    let timeout_duration = Duration::from_secs(effective_timeout);

    let spawn_result = if cfg!(target_os = "windows") {
        // Convert bash-style && to PowerShell-style ; for command chaining.
        let ps_command = command.replace(" && ", "; ");

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
        Ok(Err(e)) => return Err(anyhow!("Failed to spawn command: {}", e)),
        Err(_) => {
            warn!(
                command = %command,
                timeout_secs = effective_timeout,
                "Shell command timed out"
            );
            return Err(anyhow!(
                "Command timed out after {}s: {}",
                effective_timeout,
                command
            ));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        Ok(format!("Stdout: {}\nStderr: {}", stdout, stderr))
    } else {
        Ok(format!(
            "Command failed with code {}:\nStdout: {}\nStderr: {}",
            output.status, stdout, stderr
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acp::security::SecurityPolicy;

    #[tokio::test]
    async fn echo_command_succeeds() {
        let policy = SecurityPolicy::new();
        let result = run_shell_command("echo hello", &policy, 0).await;
        assert!(result.is_ok(), "echo should succeed: {:?}", result);
        let out = result.unwrap();
        assert!(
            out.contains("hello"),
            "output should contain 'hello': {}",
            out
        );
    }

    #[tokio::test]
    async fn blocked_command_is_rejected() {
        let policy = SecurityPolicy::new();
        // "rm -rf" is on the denylist; must be rejected before spawning.
        let result = run_shell_command("rm -rf /tmp/should_not_exist", &policy, 0).await;
        assert!(result.is_err(), "dangerous command should be blocked");
    }
}
