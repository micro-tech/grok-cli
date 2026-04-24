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
/// - Execution is bounded by [`SHELL_COMMAND_TIMEOUT_SECS`]; if the process
///   does not finish in time an error is returned (the child is killed by the
///   OS when the `Command` future is dropped).
///
/// # Errors
/// Returns an error if the command is on the denylist, fails to spawn, or
/// exceeds the timeout.
pub async fn run_shell_command(command: &str, security: &SecurityPolicy) -> Result<String> {
    security.validate_shell_command(command)?;

    let cwd = security.working_directory().to_path_buf();
    let timeout_secs = effective_timeout(security);
    let timeout_duration = Duration::from_secs(timeout_secs);

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
    async fn blocked_command_is_rejected() {
        let policy = SecurityPolicy::new();
        // "rm -rf" is on the denylist; must be rejected before spawning.
        let result = run_shell_command("rm -rf /tmp/should_not_exist", &policy).await;
        assert!(result.is_err(), "dangerous command should be blocked");
    }
}
