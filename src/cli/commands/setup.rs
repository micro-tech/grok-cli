//! Interactive setup wizard for grok-cli.
//!
//! This module is the **Terminal Auth** entry point declared in the ACP
//! `initialize` response:
//!
//! ```json
//! { "id": "grok-setup", "type": "terminal", "args": ["setup"] }
//! ```
//!
//! ACP clients such as Zed launch `grok setup` when the user has not yet
//! configured an API key.  The wizard:
//!
//! 1. Displays a welcome banner
//! 2. Checks whether `GROK_API_KEY` is already set
//! 3. Prompts the user to paste their xAI API key (input hidden)
//! 4. Validates the key looks well-formed
//! 5. Tests the key against the xAI API (with Starlink-resilient retries)
//! 6. Saves the key to `~/.grok/.env` so every future grok invocation picks
//!    it up automatically
//! 7. Prints a success summary and next-steps hint

use anyhow::{Context, Result, anyhow};
use colored::*;
use std::io::{self, Write};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the interactive setup wizard.
///
/// Called from `src/cli/app.rs` when the user runs `grok setup`.
pub async fn handle_setup() -> Result<()> {
    print_banner();

    // ------------------------------------------------------------------
    // Step 1: Check if already configured
    // ------------------------------------------------------------------
    let existing_key = resolve_existing_key();
    if let Some(ref key) = existing_key {
        let masked = mask_key(key);
        println!(
            "{}  An API key is already configured: {}",
            "✓".green().bold(),
            masked.dimmed()
        );
        println!();
        print!("{}", "Do you want to replace it? [y/N] ".yellow());
        io::stdout().flush().ok();

        let mut answer = String::new();
        io::stdin()
            .read_line(&mut answer)
            .context("Failed to read user input")?;

        if !answer.trim().eq_ignore_ascii_case("y") {
            println!();
            println!("{} Setup cancelled — using existing key.", "ℹ".cyan());
            print_next_steps();
            return Ok(());
        }
        println!();
    }

    // ------------------------------------------------------------------
    // Step 2: Ask for the API key
    // ------------------------------------------------------------------
    println!("{}", "Where to get your API key:".bold());
    println!(
        "  {}  Visit {} and create an API key.",
        "→".cyan(),
        "https://console.x.ai/".underline().cyan()
    );
    println!();

    let api_key = prompt_api_key()?;

    // ------------------------------------------------------------------
    // Step 3: Basic format validation
    // ------------------------------------------------------------------
    if let Err(e) = validate_key_format(&api_key) {
        println!();
        println!("{} {}", "✗".red().bold(), e);
        println!(
            "  {}",
            "xAI API keys usually start with 'xai-' and are at least 32 characters.".dimmed()
        );
        return Err(anyhow!("Invalid API key format"));
    }

    // ------------------------------------------------------------------
    // Step 4: Live API test (Starlink-resilient)
    // ------------------------------------------------------------------
    println!();
    print!("{}", "  Testing API key against xAI… ".dimmed());
    io::stdout().flush().ok();

    match test_api_key(&api_key).await {
        Ok(model) => {
            println!("{}", "✓ OK".green().bold());
            println!(
                "  {}",
                format!("Connected to xAI — default model: {model}").dimmed()
            );
        }
        Err(e) => {
            println!("{}", "⚠ warning".yellow().bold());
            println!(
                "  {} {}",
                "Could not verify key online:".yellow(),
                e.to_string().dimmed()
            );
            println!(
                "  {}",
                "Key will be saved anyway — check your network or key if things don't work."
                    .dimmed()
            );
        }
    }

    // ------------------------------------------------------------------
    // Step 5: Save the key
    // ------------------------------------------------------------------
    println!();
    print!("{}", "  Saving key… ".dimmed());
    io::stdout().flush().ok();

    let saved_path = save_api_key(&api_key).context("Failed to save API key")?;
    println!("{}", "✓ saved".green().bold());
    println!(
        "  {}",
        format!("Stored in: {}", saved_path.display()).dimmed()
    );

    // ------------------------------------------------------------------
    // Step 6: Success
    // ------------------------------------------------------------------
    println!();
    println!("{}", "━".repeat(60).dimmed());
    println!("{}", "  ✅  grok-cli is ready!".green().bold());
    println!("{}", "━".repeat(60).dimmed());
    println!();
    print_next_steps();

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Print the setup wizard banner.
fn print_banner() {
    println!();
    println!("{}", "━".repeat(60).cyan());
    println!("{}", "  grok-cli  ·  Interactive Setup Wizard".bold());
    println!("{}", "━".repeat(60).cyan());
    println!();
    println!(
        "  This wizard will configure your {} so that",
        "xAI API key".bold()
    );
    println!("  grok-cli and the Zed editor integration work correctly.");
    println!();
}

/// Check all the places grok-cli looks for an API key and return the first
/// non-empty value found, without loading the full Config (setup runs before
/// any config is guaranteed to be valid).
fn resolve_existing_key() -> Option<String> {
    // 1. Environment variable already set by the shell or a loaded .env
    if let Ok(k) = std::env::var("GROK_API_KEY")
        && !k.trim().is_empty()
    {
        return Some(k.trim().to_string());
    }

    // 2. ~/.grok/.env file
    if let Some(home) = dirs::home_dir() {
        let env_path = home.join(".grok").join(".env");
        if env_path.exists()
            && let Ok(content) = std::fs::read_to_string(&env_path)
        {
            for line in content.lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("GROK_API_KEY=") {
                    let value = rest.trim().trim_matches('"');
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Prompt the user to paste an API key.  The input is read as plain text
/// (terminals used inside editors like Zed may not support hidden input),
/// but we do not echo it back in any summary line.
fn prompt_api_key() -> Result<String> {
    println!("{}", "Paste your xAI API key below and press Enter.".bold());
    println!(
        "  {}",
        "(The key will not be displayed as you type on most terminals)".dimmed()
    );
    println!();
    print!("  API key: ");
    io::stdout().flush().ok();

    // Attempt to read without echo using the `rpassword` approach.
    // Fall back to plain stdin if that's not available (e.g. inside a pipe).
    let key = read_secret_line()?;

    let key = key.trim().to_string();
    if key.is_empty() {
        return Err(anyhow!("No API key entered."));
    }

    Ok(key)
}

/// Read a line from stdin.  On interactive terminals we disable echo so the
/// key is not shown; inside pipes / non-TTY environments we fall back to a
/// normal `read_line`.
fn read_secret_line() -> Result<String> {
    // Try the cross-platform "disable echo" approach.
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        // Only try to disable echo if stdin is actually a TTY.
        if unsafe { libc_isatty(io::stdin().as_raw_fd()) } {
            if let Ok(key) = read_without_echo_unix() {
                println!(); // newline after hidden input
                return Ok(key);
            }
        }
    }

    // Fallback: plain read_line (works in all environments).
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .context("Failed to read API key from stdin")?;
    Ok(line)
}

/// On Unix, temporarily disable terminal echo to hide the key as it's typed.
#[cfg(unix)]
fn read_without_echo_unix() -> Result<String> {
    use std::io::BufRead;

    // Use `stty -echo` / `stty echo` around the read.
    let _ = std::process::Command::new("stty").arg("-echo").status();

    let mut line = String::new();
    let result = io::BufReader::new(io::stdin()).read_line(&mut line);

    let _ = std::process::Command::new("stty").arg("echo").status();

    result.context("Failed to read API key")?;
    Ok(line)
}

#[cfg(unix)]
fn libc_isatty(fd: std::os::unix::io::RawFd) -> bool {
    // SAFETY: isatty is well-defined for all fd values; it simply returns 0
    // for non-TTY file descriptors.
    unsafe { libc::isatty(fd) != 0 }
}

/// Light format check — rejects obviously wrong values without a network call.
fn validate_key_format(key: &str) -> Result<()> {
    let key = key.trim();

    if key.len() < 32 {
        return Err(anyhow!(
            "Key is too short ({} chars). Expected at least 32 characters.",
            key.len()
        ));
    }

    if key.contains(char::is_whitespace) {
        return Err(anyhow!("Key must not contain whitespace."));
    }

    // xAI keys currently start with "xai-" — warn but don't block.
    if !key.starts_with("xai-") {
        eprintln!(
            "  {} Key does not start with 'xai-'. Double-check it is an xAI key.",
            "⚠".yellow()
        );
    }

    Ok(())
}

/// Attempt a minimal API call to verify the key works.
///
/// Uses exponential back-off so a Starlink satellite handover during setup
/// does not permanently fail validation.
async fn test_api_key(api_key: &str) -> Result<String> {
    const MAX_RETRIES: u32 = 3;
    const BASE_DELAY_SECS: u64 = 3;

    let mut last_err = anyhow!("No attempts made");

    for attempt in 1..=MAX_RETRIES {
        match try_api_call(api_key).await {
            Ok(model) => return Ok(model),
            Err(e) => {
                last_err = e;
                let is_auth_error = last_err.to_string().to_lowercase().contains("401")
                    || last_err.to_string().to_lowercase().contains("unauthorized")
                    || last_err.to_string().to_lowercase().contains("forbidden");

                // Authentication errors are permanent — no point retrying.
                if is_auth_error {
                    return Err(anyhow!(
                        "API key rejected (401 Unauthorized). \
                         Please double-check the key at https://console.x.ai/"
                    ));
                }

                if attempt < MAX_RETRIES {
                    let delay = BASE_DELAY_SECS * (1 << (attempt - 1)); // 3 → 6 → 12 s
                    eprintln!(
                        "\n  {} Attempt {}/{} failed ({}). Retrying in {}s…",
                        "⚠".yellow(),
                        attempt,
                        MAX_RETRIES,
                        last_err,
                        delay
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
            }
        }
    }

    Err(last_err)
}

/// Single attempt at the xAI models endpoint — cheap, no tokens consumed.
async fn try_api_call(api_key: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .context("Failed to create HTTP client")?;

    let resp = client
        .get("https://api.x.ai/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .send()
        .await
        .context("Network request failed")?;

    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(anyhow!("401 Unauthorized"));
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "API error {}: {}",
            status,
            body.chars().take(120).collect::<String>()
        ));
    }

    // Parse the model list and return the first model ID as a friendly hint.
    let body: serde_json::Value = resp.json().await.context("Failed to parse API response")?;
    let first_model = body
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|m| m.get("id"))
        .and_then(|id| id.as_str())
        .unwrap_or("grok-3")
        .to_string();

    Ok(first_model)
}

/// Mask the key for display: show only the first 8 and last 4 characters.
fn mask_key(key: &str) -> String {
    let key = key.trim();
    if key.len() <= 16 {
        return "*".repeat(key.len());
    }
    let prefix = &key[..8];
    let suffix = &key[key.len() - 4..];
    let masked = "*".repeat(key.len().saturating_sub(12));
    format!("{prefix}{masked}{suffix}")
}

/// Write `GROK_API_KEY=<key>` to `~/.grok/.env`, creating the directory and
/// file if necessary.  Any existing `GROK_API_KEY=` line is replaced; other
/// lines are preserved.
fn save_api_key(api_key: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let grok_dir = home.join(".grok");

    // Ensure ~/.grok/ exists.
    std::fs::create_dir_all(&grok_dir)
        .with_context(|| format!("Failed to create directory: {}", grok_dir.display()))?;

    let env_path = grok_dir.join(".env");
    let key_line = format!("GROK_API_KEY=\"{}\"", api_key);

    // Read existing content (if any) and replace/append the key line.
    let existing = if env_path.exists() {
        std::fs::read_to_string(&env_path)
            .with_context(|| format!("Failed to read: {}", env_path.display()))?
    } else {
        String::new()
    };

    let mut lines: Vec<String> = existing
        .lines()
        .filter(|l| !l.trim_start().starts_with("GROK_API_KEY="))
        .map(str::to_string)
        .collect();

    lines.push(key_line);

    // Ensure trailing newline.
    let mut content = lines.join("\n");
    content.push('\n');

    std::fs::write(&env_path, &content)
        .with_context(|| format!("Failed to write: {}", env_path.display()))?;

    // Restrict permissions on Unix so the key is not world-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&env_path, perms)
            .with_context(|| format!("Failed to set permissions on: {}", env_path.display()))?;
    }

    Ok(env_path)
}

/// Print the "what to do next" hint shown at the end of a successful setup.
fn print_next_steps() {
    println!("{}", "Next steps:".bold());
    println!(
        "  {}  Open a project in Zed — grok-cli is ready as your AI agent.",
        "→".cyan()
    );
    println!(
        "  {}  Or run {} for interactive AI chat in the terminal.",
        "→".cyan(),
        "grok".bold()
    );
    println!(
        "  {}  Run {} to verify everything is working.",
        "→".cyan(),
        "grok health --all".bold()
    );
    println!();
    println!(
        "  {} {}",
        "Docs:".dimmed(),
        "https://github.com/micro-tech/grok-cli"
            .underline()
            .dimmed()
    );
    println!(
        "  {} {}",
        "Buy me a coffee:".dimmed(),
        "https://buymeacoffee.com/micro.tech".underline().dimmed()
    );
    println!();
}
