//! Interactive setup wizard — the ACP **Terminal Auth** entry point.
//!
//! Declared in the ACP `initialize` response as:
//! ```json
//! { "id": "grok-setup", "type": "terminal", "args": ["setup"] }
//! ```
//!
//! ACP clients such as Zed invoke `grok setup` when no API key is configured.
//! The wizard:
//!
//! 1. Greets the user and explains what it does.
//! 2. Checks whether a key is already configured.
//! 3. Prompts for the xAI API key with masked (starred) input via crossterm.
//! 4. Validates the key format.
//! 5. Tests the key against the xAI API with Starlink-resilient retries.
//! 6. Persists the key to `~/.grok/.env` (loaded on every startup).
//! 7. Prints success message and next-step hints.

use anyhow::{Context, Result, anyhow};
use colored::*;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self as crossterm_terminal, ClearType},
};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum API verification retries (Starlink can drop for 20-30 s).
const MAX_VERIFY_RETRIES: u32 = 4;

/// Base delay between retries in seconds (doubles each attempt).
const RETRY_BASE_SECS: u64 = 3;

/// HTTP timeout per verification attempt.
const VERIFY_TIMEOUT_SECS: u64 = 18;

// ── Public entry point ────────────────────────────────────────────────────────

/// Run the interactive Terminal Auth setup wizard.
///
/// Called from `src/cli/app.rs` when the user runs `grok setup`.
pub async fn handle_setup() -> Result<()> {
    print_banner();

    // ── Step 1: Check for an existing key ────────────────────────────────────
    if let Some(existing_key) = resolve_existing_key() {
        let masked = mask_key(&existing_key);
        println!(
            "{}  An API key is already configured: {}",
            "✓".green().bold(),
            masked.dimmed()
        );
        println!();

        if !ask_yes_no("Replace it with a new key?")? {
            println!();
            println!("{} Setup cancelled — existing key kept.", "ℹ".cyan());
            print_next_steps();
            return Ok(());
        }
        println!();
    }

    // ── Step 2: Where to get the key ─────────────────────────────────────────
    println!("{}", "Where to get your API key:".bold());
    println!(
        "  {} Visit {}",
        "→".cyan(),
        "https://console.x.ai/".underline().cyan()
    );
    println!("    Sign in, open the API section, and create a new key.");
    println!();

    // ── Step 3: Prompt for the key with masked input ──────────────────────────
    let api_key = prompt_masked_key()?;

    if api_key.is_empty() {
        return Err(anyhow!("No API key entered. Setup cancelled."));
    }

    // ── Step 4: Basic format check ────────────────────────────────────────────
    validate_key_format(&api_key)?;

    // ── Step 5: Live verification (Starlink-resilient) ────────────────────────
    println!();
    print!("[1/2] {}", "Verifying key with xAI API…".dimmed());
    io::stdout().flush().ok();

    match verify_api_key_with_retries(&api_key).await {
        Ok(model_hint) => {
            // Overwrite the line so progress text is replaced by success.
            let _ = execute!(
                io::stdout(),
                cursor::MoveToColumn(0),
                crossterm::terminal::Clear(ClearType::CurrentLine)
            );
            println!(
                "[1/2] {} {}",
                "✓ Key verified!".green().bold(),
                format!("(model: {})", model_hint).dimmed()
            );
        }
        Err(e) => {
            println!();
            println!(
                "  {} Could not verify key online: {}",
                "⚠".yellow().bold(),
                e.to_string().dimmed()
            );

            // Fatal only for explicit auth failures; network errors are
            // non-fatal so the user can still save and try later.
            let msg = e.to_string().to_lowercase();
            if msg.contains("401") || msg.contains("unauthori") || msg.contains("invalid api key") {
                println!();
                println!(
                    "  {} The xAI API rejected this key. Please double-check it at {}",
                    "✗".red().bold(),
                    "https://console.x.ai/".cyan()
                );
                return Err(anyhow!("API key rejected by xAI."));
            }

            println!(
                "  {}",
                "Key will be saved anyway — check your network if issues persist.".dimmed()
            );
        }
    }

    // ── Step 6: Save the key ──────────────────────────────────────────────────
    print!("[2/2] {}", "Saving key to ~/.grok/.env…".dimmed());
    io::stdout().flush().ok();

    let saved_path = save_api_key(&api_key).context("Failed to save API key")?;
    let _ = execute!(
        io::stdout(),
        cursor::MoveToColumn(0),
        crossterm::terminal::Clear(ClearType::CurrentLine)
    );
    println!(
        "[2/2] {} {}",
        "✓ Saved!".green().bold(),
        format!("({})", saved_path.display()).dimmed()
    );

    // ── Done ──────────────────────────────────────────────────────────────────
    println!();
    println!("{}", "━".repeat(60).dimmed());
    println!("{}", "  ✅  grok-cli is ready to use!".green().bold());
    println!("{}", "━".repeat(60).dimmed());
    println!();
    print_next_steps();

    Ok(())
}

// ── Prompts ───────────────────────────────────────────────────────────────────

/// Prompt the user to paste their API key.  The input is masked with `*`
/// characters using crossterm raw mode so the key is never echoed.
///
/// Falls back to plain `read_line` if raw mode cannot be enabled (e.g. when
/// stdin is a pipe in a non-interactive environment).
fn prompt_masked_key() -> Result<String> {
    println!("{}", "Paste your xAI API key and press Enter.".bold());
    println!(
        "  {}",
        "(Characters will be shown as  *  as you type)".dimmed()
    );
    println!();
    print!("  {} ", "API key:".bold());
    io::stdout().flush()?;

    // Attempt raw-mode masked input.
    match read_masked() {
        Ok(key) => Ok(key.trim().to_string()),
        Err(_) => {
            // Raw mode unavailable (CI / pipe) — fall back to plain stdin.
            eprintln!();
            eprintln!(
                "  {}",
                "(Raw mode unavailable — key will be visible as you type)".yellow()
            );
            print!("  API key: ");
            io::stdout().flush()?;
            let mut buf = String::new();
            io::stdin()
                .read_line(&mut buf)
                .context("Failed to read API key from stdin")?;
            Ok(buf.trim().to_string())
        }
    }
}

/// Read a single line from stdin using crossterm raw mode, echoing `*` for
/// each character typed.  Supports Backspace and Ctrl-C.
fn read_masked() -> Result<String> {
    crossterm_terminal::enable_raw_mode()?;

    let mut key = String::new();
    let mut stdout = io::stdout();

    let result: Result<String> = loop {
        // poll with a short timeout so we don't spin forever
        if event::poll(Duration::from_millis(200))?
            && let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event::read()?
        {
            match code {
                // Finish on Enter
                KeyCode::Enter => {
                    break Ok(key);
                }
                // Abort on Ctrl-C / Ctrl-D
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    break Err(anyhow!("Setup cancelled (Ctrl-C)."));
                }
                KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                    break Err(anyhow!("Setup cancelled (Ctrl-D)."));
                }
                // Backspace: erase last char
                KeyCode::Backspace | KeyCode::Delete => {
                    if key.pop().is_some() {
                        // Move back, overwrite with space, move back again
                        write!(stdout, "\x08 \x08")?;
                        stdout.flush()?;
                    }
                }
                // Regular character
                KeyCode::Char(c) => {
                    key.push(c);
                    write!(stdout, "*")?;
                    stdout.flush()?;
                }
                _ => {}
            }
        }
    };

    // Always restore terminal before returning, even on error.
    let _ = crossterm_terminal::disable_raw_mode();
    // Move to next line after the masked field.
    writeln!(stdout)?;
    stdout.flush()?;

    result
}

/// Ask a simple yes/no question.  Returns `true` for "y" / "yes".
fn ask_yes_no(question: &str) -> Result<bool> {
    print!("{} {} [y/N]: ", "?".cyan().bold(), question);
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .context("Failed to read user input")?;
    Ok(matches!(buf.trim().to_lowercase().as_str(), "y" | "yes"))
}

// ── Validation ────────────────────────────────────────────────────────────────

/// Light format check — rejects obviously wrong values without a network call.
fn validate_key_format(key: &str) -> Result<()> {
    let key = key.trim();

    if key.len() < 20 {
        return Err(anyhow!(
            "Key looks too short ({} chars, expected ≥ 20). \
             Make sure you copied the full key from https://console.x.ai/",
            key.len()
        ));
    }

    if key.contains(char::is_whitespace) {
        return Err(anyhow!(
            "Key contains whitespace. Paste the key exactly as shown in the console."
        ));
    }

    if !key.starts_with("xai-") {
        // Warn but don't block — key format could change.
        eprintln!(
            "  {} Key does not start with 'xai-'. \
             Double-check it is an xAI key from https://console.x.ai/",
            "⚠".yellow()
        );
    }

    Ok(())
}

// ── API Verification ──────────────────────────────────────────────────────────

/// Verify the key against the xAI `/v1/models` endpoint.
///
/// Uses exponential back-off to handle Starlink satellite handovers which
/// can cause 20-30 second connection drops.  Returns the first model ID as
/// a friendly confirmation string.
async fn verify_api_key_with_retries(api_key: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(VERIFY_TIMEOUT_SECS))
        .build()
        .context("Failed to build HTTP client")?;

    let mut last_err = anyhow!("No verification attempts made");

    for attempt in 1..=MAX_VERIFY_RETRIES {
        match try_verify_once(&client, api_key).await {
            Ok(model) => return Ok(model),
            Err(e) => {
                let msg = e.to_string().to_lowercase();

                // Auth errors are permanent — bail immediately.
                if msg.contains("401") || msg.contains("unauthori") {
                    return Err(anyhow!("401 Unauthorized — API key rejected by xAI."));
                }

                last_err = e;
                info!(
                    "API key verification attempt {}/{} failed: {}",
                    attempt, MAX_VERIFY_RETRIES, last_err
                );

                if attempt < MAX_VERIFY_RETRIES {
                    let delay_secs = RETRY_BASE_SECS * (1 << (attempt - 1)); // 3, 6, 12
                    eprintln!(
                        "\n  {} Attempt {}/{} failed. Retrying in {}s… (network drop?)",
                        "⚠".yellow(),
                        attempt,
                        MAX_VERIFY_RETRIES,
                        delay_secs
                    );
                    tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                    // Reprint the progress indicator on the next line.
                    print!("  {}", "Re-verifying…".dimmed());
                    io::stdout().flush().ok();
                }
            }
        }
    }

    Err(last_err)
}

/// Single attempt: GET `/v1/models` with the Bearer token.
async fn try_verify_once(client: &reqwest::Client, api_key: &str) -> Result<String> {
    let resp = client
        .get("https://api.x.ai/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .context("Network request to api.x.ai failed")?;

    let status = resp.status();

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(anyhow!("401 Unauthorized"));
    }

    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "HTTP {} — {}",
            status,
            body.chars().take(120).collect::<String>()
        ));
    }

    // Pull the first model ID as a user-friendly confirmation.
    let payload: serde_json::Value = resp.json().await.context("Failed to parse API response")?;
    let first_model = payload
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|m| m.get("id"))
        .and_then(|id| id.as_str())
        .unwrap_or("grok-3")
        .to_string();

    Ok(first_model)
}

// ── Key persistence ───────────────────────────────────────────────────────────

/// Write `GROK_API_KEY="<key>"` to `~/.grok/.env`.
///
/// Creates the directory and file if they do not exist.  Any existing
/// `GROK_API_KEY=` line is replaced in-place; all other lines are preserved.
///
/// On Unix the file is chmod'd to 0600 so the key is not world-readable.
fn save_api_key(api_key: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let grok_dir = home.join(".grok");

    std::fs::create_dir_all(&grok_dir)
        .with_context(|| format!("Failed to create directory: {}", grok_dir.display()))?;

    let env_path = grok_dir.join(".env");
    let new_line = format!("GROK_API_KEY=\"{}\"", api_key);

    // Read and rewrite, replacing any existing GROK_API_KEY line.
    let existing = if env_path.exists() {
        std::fs::read_to_string(&env_path)
            .with_context(|| format!("Failed to read {}", env_path.display()))?
    } else {
        "# grok-cli environment — generated by `grok setup`\n".to_string()
    };

    let mut replaced = false;
    let mut lines: Vec<String> = existing
        .lines()
        .map(|l| {
            if l.trim_start().starts_with("GROK_API_KEY=") {
                replaced = true;
                new_line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();

    if !replaced {
        lines.push(new_line);
    }

    let mut content = lines.join("\n");
    content.push('\n');

    std::fs::write(&env_path, &content)
        .with_context(|| format!("Failed to write {}", env_path.display()))?;

    // Restrict permissions on Unix (no-op on Windows).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&env_path, perms)
            .with_context(|| format!("Failed to set permissions on {}", env_path.display()))?;
    }

    info!("API key saved to {}", env_path.display());
    Ok(env_path)
}

// ── Existing-key detection ────────────────────────────────────────────────────

/// Check all standard locations for an existing API key without loading the
/// full Config stack (the wizard may run before any config exists).
fn resolve_existing_key() -> Option<String> {
    // 1. Environment variable
    if let Ok(k) = std::env::var("GROK_API_KEY") {
        let k = k.trim().to_string();
        if !k.is_empty() {
            return Some(k);
        }
    }

    // 2. ~/.grok/.env
    let env_path = dirs::home_dir()?.join(".grok").join(".env");
    if let Ok(content) = std::fs::read_to_string(&env_path) {
        for line in content.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("GROK_API_KEY=") {
                let val = val.trim().trim_matches('"');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }

    None
}

// ── Display helpers ───────────────────────────────────────────────────────────

fn print_banner() {
    println!();
    println!("{}", "━".repeat(60).cyan());
    println!("{}", "  🤖  grok-cli · Setup Wizard".bold());
    println!("{}", "━".repeat(60).cyan());
    println!();
    println!(
        "  This wizard configures your {} so that",
        "xAI API key".bold()
    );
    println!("  grok-cli (and the Zed Grok agent) can authenticate.");
    println!();
}

/// Mask a key: show first 8 chars, stars for the middle, last 4 chars.
fn mask_key(key: &str) -> String {
    let key = key.trim();
    if key.len() <= 16 {
        return "*".repeat(key.len());
    }
    let prefix = &key[..8];
    let suffix = &key[key.len() - 4..];
    let stars = "*".repeat(key.len().saturating_sub(12));
    format!("{}{}{}", prefix, stars, suffix)
}

fn print_next_steps() {
    println!("{}", "Next steps:".bold());
    println!(
        "  {} Open a project in Zed — the Grok agent is ready.",
        "→".cyan()
    );
    println!(
        "  {} Run {} for interactive AI chat in the terminal.",
        "→".cyan(),
        "grok".bold()
    );
    println!(
        "  {} Run {} to verify everything works.",
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
        "☕ Buy me a coffee:".dimmed(),
        "https://buymeacoffee.com/micro.tech".underline().dimmed()
    );
    println!();
}
