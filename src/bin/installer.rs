use colored::*;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use winreg::RegKey;
#[cfg(windows)]
use winreg::enums::*;

fn find_project_root() -> Option<PathBuf> {
    let mut current = env::current_dir().ok()?;
    loop {
        if current.join("Cargo.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn get_version(root_dir: &Path) -> String {
    let cargo_toml = root_dir.join("Cargo.toml");
    if let Ok(content) = fs::read_to_string(&cargo_toml) {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("version") && trimmed.contains('=') {
                if let Some(v) = trimmed.split('=').nth(1) {
                    let v = v.trim().trim_matches('"').trim_matches('\'');
                    if !v.is_empty() {
                        return v.to_string();
                    }
                }
            }
        }
    }
    "unknown".to_string()
}

fn main() {
    println!("{}", "Grok-CLI Installer for Windows 11".green().bold());
    println!("=============================================");

    #[cfg(windows)]
    {
        let root_dir =
            find_project_root().expect("Failed to find project root (Cargo.toml not found)");
        let version = get_version(&root_dir);
        println!(
            "{}",
            format!("Installing v{} for Windows 11", version)
                .green()
                .bold()
        );
        install_windows(root_dir);
    }
}

#[cfg(windows)]
fn install_windows(root_dir: PathBuf) {
    // 0. Migrate from old .grok directory (if user previously used the old grok project)
    migrate_from_old_grok_directory();

    // 1. Check for old Cargo installation
    check_and_remove_old_cargo_install();

    // 1. Build the release binary
    println!("{}", "Building release binary...".cyan());
    let status = Command::new("cargo")
        .current_dir(&root_dir)
        .args(["build", "--release", "--bin", "grok-cli"])
        .status()
        .expect("Failed to execute cargo build");

    if !status.success() {
        eprintln!("{}", "Build failed. Aborting installation.".red());
        std::process::exit(1);
    }

    // 2. Define paths
    let local_app_data = env::var("LOCALAPPDATA").expect("LOCALAPPDATA not set");
    let install_dir = PathBuf::from(&local_app_data).join("grok-cli").join("bin");
    let exe_name = "grok-cli.exe";
    let target_exe = install_dir.join(exe_name);

    let source_exe = root_dir.join("target").join("release").join(exe_name);

    if !source_exe.exists() {
        eprintln!(
            "{} {}",
            "Source binary not found at:".red(),
            source_exe.display()
        );
        std::process::exit(1);
    }

    // 3. Create directory
    if !install_dir.exists() {
        println!("Creating installation directory: {}", install_dir.display());
        fs::create_dir_all(&install_dir).expect("Failed to create installation directory");
    }

    // 4. Remove old binary if it exists
    if target_exe.exists() {
        println!("Removing old installation...");
        if let Err(e) = fs::remove_file(&target_exe) {
            eprintln!("{} {}", "Failed to remove old binary:".red(), e);
            eprintln!(
                "{}",
                "Please make sure 'grok-cli.exe' is not currently running.".yellow()
            );
            std::process::exit(1);
        }
    }

    // 5. Copy binary
    println!("Copying binary to {}", target_exe.display());
    fs::copy(&source_exe, &target_exe).expect("Failed to copy binary");

    // 6. Copy additional files (LICENSE, docs, examples)
    println!("{}", "Installing documentation and examples...".cyan());
    install_additional_files(&root_dir, &install_dir);

    // 7. Update PATH
    println!("{}", "Updating PATH environment variable...".cyan());
    update_path(&install_dir);

    // 8. Create Start Menu Shortcut
    println!("{}", "Creating Start Menu shortcut...".cyan());
    create_shortcut(&target_exe);

    // 9. Setup Configuration
    println!("{}", "Setting up configuration...".cyan());
    setup_config(&root_dir);

    // 10. Setup Global Context
    println!("{}", "Setting up global context...".cyan());
    setup_context(&root_dir);

    // 11. Setup Audit Directory
    println!("{}", "Setting up audit logging...".cyan());
    setup_audit_directory();

    // 12. Setup Session DNA
    println!("{}", "Setting up Session DNA...".cyan());
    setup_session_dna(&root_dir);

    // 13. Install agent presets
    println!("{}", "Installing agent presets...".cyan());
    setup_agent_presets(&root_dir);

    println!("\n{}", "Installation Complete!".green().bold());
    let version = get_version(&root_dir);
    println!("Version: {}", version);
    println!("\nNew features in this version:");
    println!(
        "  • Full per-agent runtime: SubAgentConfig with tool permissions, persona, safety, context budget & sandbox"
    );
    println!(
        "  • Agent presets installed: planner, coder, researcher, verifier (.grok-cli/agents/)"
    );
    println!("  • grok sandbox — isolated playground workspace with pre-populated Rust project");
    println!("  • ACP session/load bug fixed — Zed sessions now initialise correctly");
    println!("  • fork_agent now runs real parallel xAI API calls via tokio::spawn");
    println!("  • Sub-agent tool loop: spawn_agent_configured with SecurityPolicy scoping");
    println!("\nPlease restart your terminal to use the 'grok-cli' command.");
    println!(
        "{}",
        "IMPORTANT: The installed command is 'grok-cli' (not 'grok')."
            .yellow()
            .bold()
    );
    println!(
        "{}",
        "This avoids conflict with the official XAI Grok BUILD that also installs as 'grok'."
            .yellow()
    );
    println!(
        "\nDocumentation installed to: {}",
        install_dir.parent().unwrap().join("docs").display()
    );
    println!(
        "View README: {}",
        install_dir
            .parent()
            .unwrap()
            .join("docs")
            .join("README.md")
            .display()
    );
}

#[cfg(windows)]
fn migrate_from_old_grok_directory() {
    if let Some(home_dir) = dirs::home_dir() {
        let old_dir = home_dir.join(".grok");
        let new_dir = home_dir.join(".grok-cli");

        if old_dir.exists() && !new_dir.exists() {
            println!(
                "{}",
                "Old ~/.grok directory detected. Migrating to ~/.grok-cli...".yellow()
            );

            match fs::rename(&old_dir, &new_dir) {
                Ok(_) => {
                    println!(
                        "{}",
                        "✓ Successfully migrated ~/.grok → ~/.grok-cli".green()
                    );
                }
                Err(e) => {
                    eprintln!("{} {}", "Failed to rename old directory:".red(), e);
                    println!(
                        "  Please manually move contents from {} to {}",
                        old_dir.display(),
                        new_dir.display()
                    );
                }
            }
            println!();
        } else if old_dir.exists() && new_dir.exists() {
            println!("{}", "Both ~/.grok and ~/.grok-cli exist..".yellow());
            println!();
        }
    }
}

#[cfg(windows)]
fn check_and_remove_old_cargo_install() {
    if let Some(home_dir) = dirs::home_dir() {
        let cargo_grok_cli = home_dir.join(".cargo").join("bin").join("grok-cli.exe");

        if cargo_grok_cli.exists() {
            println!("\n{}", "Old Cargo installation detected!".yellow().bold());
            println!("Found old version at: {}", cargo_grok_cli.display());

            // Try to get version
            if let Ok(output) = Command::new(&cargo_grok_cli).arg("--version").output()
                && let Ok(version) = String::from_utf8(output.stdout)
            {
                println!("Old version: {}", version.trim());
            }

            print!(
                "\n{}",
                "Do you want to remove the old installation? (yes/no): ".yellow()
            );
            io::stdout().flush().unwrap();

            let mut response = String::new();
            io::stdin().read_line(&mut response).unwrap();

            if response.trim().eq_ignore_ascii_case("yes") {
                match fs::remove_file(&cargo_grok_cli) {
                    Ok(_) => {
                        println!(
                            "{}",
                            "✓ Old Cargo installation removed successfully!".green()
                        );
                        println!(
                            "{}",
                            "  You may need to restart your terminal after installation.".cyan()
                        );
                    }
                    Err(e) => {
                        eprintln!("{}", "Failed to remove old installation:".red());
                        eprintln!("  {}", e);
                        eprintln!(
                            "{}",
                            "  Please close all running grok-cli instances and try again.".yellow()
                        );
                        eprintln!("{}", "  Or manually delete: ".yellow());
                        eprintln!("  {}", cargo_grok_cli.display());
                    }
                }
            } else {
                println!(
                    "{}",
                    "Skipping removal. You may have version conflicts.".yellow()
                );
                println!("{}", "  To remove later, delete: ".cyan());
                println!("  {}", cargo_grok_cli.display());
            }
            println!();
        }
    }
}

#[cfg(windows)]
fn setup_context(root_dir: &Path) {
    if let Some(home_dir) = dirs::home_dir() {
        let grok_cli_dir = home_dir.join(".grok-cli");
        if !grok_cli_dir.exists()
            && let Err(e) = fs::create_dir_all(&grok_cli_dir)
        {
            eprintln!("Failed to create .grok-cli directory: {}", e);
            return;
        }

        let source_context = root_dir.join("context.md");
        let target_context = grok_cli_dir.join("context.md");

        if source_context.exists() {
            match fs::copy(&source_context, &target_context) {
                Ok(_) => println!("Global context installed to {}", target_context.display()),
                Err(e) => eprintln!("Failed to install global context: {}", e),
            }
        } else {
            println!("No context.md found in project root, skipping global context setup.");
        }
    } else {
        eprintln!("Failed to locate home directory for context setup.");
    }
}

#[cfg(windows)]
fn setup_memory_directories() {
    if let Some(home_dir) = dirs::home_dir() {
        let grok_cli_dir = home_dir.join(".grok-cli");

        // Directories expected by the memory, episodic, and skill modules
        let subdirs = [
            ("sessions", "Episodic memory session archives"),
            ("skills", "User skill definitions"),
            ("memory", "Long-term memory store"),
            ("traces", "RPL reasoning trace cache (optional)"),
        ];

        for (name, description) in &subdirs {
            let dir = grok_cli_dir.join(name);
            if !dir.exists() {
                match fs::create_dir_all(&dir) {
                    Ok(_) => println!("Created ~/.grok-cli/{name}/ — {description}"),
                    Err(e) => eprintln!("Failed to create ~/.grok-cli/{name}/: {e}"),
                }
            } else {
                println!("~/.grok-cli/{name}/ already exists.");
            }
        }
    } else {
        eprintln!("Failed to locate home directory for memory directory setup.");
    }
}

#[cfg(windows)]
fn setup_audit_directory() {
    if let Some(home_dir) = dirs::home_dir() {
        let audit_dir = home_dir.join(".grok-cli").join("audit");

        if !audit_dir.exists() {
            match fs::create_dir_all(&audit_dir) {
                Ok(_) => println!("Audit directory created at {}", audit_dir.display()),
                Err(e) => eprintln!("Failed to create audit directory: {}", e),
            }
        } else {
            println!("Audit directory already exists at {}", audit_dir.display());
        }
    } else {
        eprintln!("Failed to locate home directory for audit setup.");
    }
}

#[cfg(windows)]
fn setup_session_dna(root_dir: &Path) {
    if let Some(home_dir) = dirs::home_dir() {
        let grok_cli_dir = home_dir.join(".grok-cli");
        if !grok_cli_dir.exists()
            && let Err(e) = fs::create_dir_all(&grok_cli_dir)
        {
            eprintln!("Failed to create .grok-cli directory: {}", e);
            return;
        }

        let source_dna = root_dir.join("session_dna.json");
        let target_dna = grok_cli_dir.join("session_dna.json");

        if source_dna.exists() {
            match fs::copy(&source_dna, &target_dna) {
                Ok(_) => println!("Session DNA installed to {}", target_dna.display()),
                Err(e) => eprintln!("Failed to install session_dna.json: {}", e),
            }
        } else {
            println!("No session_dna.json found in project root, skipping Session DNA setup.");
        }
    } else {
        eprintln!("Failed to locate home directory for Session DNA setup.");
    }
}

#[cfg(windows)]
fn setup_config(root_dir: &Path) {
    let config_dir = dirs::config_dir()
        .expect("Failed to get config directory")
        .join("grok-cli");

    if !config_dir.exists()
        && let Err(e) = fs::create_dir_all(&config_dir)
    {
        eprintln!("Failed to create config directory: {}", e);
        return;
    }

    let config_file = config_dir.join("config.toml");
    let example_config_src = root_dir.join("config.example.toml");
    let example_config_dst = config_dir.join("config.example.toml");

    // Always copy example config for reference
    if example_config_src.exists() {
        if let Err(e) = fs::copy(&example_config_src, &example_config_dst) {
            eprintln!("Failed to copy example config: {}", e);
        } else {
            println!(
                "Example config installed to {}",
                example_config_dst.display()
            );
        }
    }

    if !config_file.exists() {
        println!("\n{}", "Setting up configuration...".cyan());
        println!("Configuration will be saved to: {}", config_file.display());

        // Copy example config as the default config.toml
        if example_config_src.exists() {
            if let Err(e) = fs::copy(&example_config_src, &config_file) {
                eprintln!("Failed to create default config: {}", e);
                return;
            }
            println!("Default configuration created from example config.");
        } else {
            eprintln!("Warning: config.example.toml not found in project root!");
            return;
        }

        // Prompt for API key
        print!("\nDo you want to set up your Grok API Key now? [Y/n]: ");
        io::stdout().flush().unwrap_or_default();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let input = input.trim().to_lowercase();
            if input != "n" && input != "no" {
                print!("Enter your X API Key (starts with 'xai-'): ");
                io::stdout().flush().unwrap_or_default();
                let mut key = String::new();
                if io::stdin().read_line(&mut key).is_ok() {
                    let key = key.trim();
                    if !key.is_empty() {
                        // Write API key to .env file (config.toml already copied from example)
                        let env_file = config_dir.join(".env");
                        let env_content = format!("GROK_API_KEY={}\n", key);
                        match fs::write(&env_file, env_content) {
                            Ok(_) => {
                                println!("{}", "✓ API key configured successfully!".green());
                                println!("API key saved to: {}", env_file.display());
                                println!(
                                    "\n{}",
                                    "Note: The .env file contains sensitive data and is excluded from version control.".yellow()
                                );
                            }
                            Err(e) => {
                                eprintln!("Failed to create .env file: {}", e);
                                println!("\nYou can manually create a .env file at:");
                                println!("  {}", env_file.display());
                                println!("And add: GROK_API_KEY={}", key);
                            }
                        }
                    } else {
                        println!("Skipping API key setup (empty key provided).");
                        println!("You can manually create a .env file later at:");
                        let env_file = config_dir.join(".env");
                        println!("  {}", env_file.display());
                        println!("And add: GROK_API_KEY=your-api-key-here");
                    }
                }
            } else {
                println!("Skipping API key setup.");
                println!("You can manually create a .env file later at:");
                let env_file = config_dir.join(".env");
                println!("  {}", env_file.display());
                println!("And add: GROK_API_KEY=your-api-key-here");
            }
        }
    } else {
        println!(
            "Configuration file already exists at: {}",
            config_file.display()
        );
    }
}

#[cfg(windows)]
fn update_path(install_dir: &Path) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env_key = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .expect("Failed to open Environment registry key");

    let current_path: String = env_key.get_value("Path").unwrap_or_default();
    let install_path_str = install_dir.to_string_lossy();

    // Simple check to avoid duplicates (case-insensitive check would be better but this is a start)
    if !current_path.contains(&*install_path_str) {
        let new_path = if current_path.is_empty() {
            install_path_str.to_string()
        } else {
            format!("{};{}", current_path, install_path_str)
        };

        env_key
            .set_value("Path", &new_path)
            .expect("Failed to update Path");
        println!("Added {} to PATH.", install_path_str);
    } else {
        println!("Path already configured.");
    }
}

#[cfg(windows)]
fn create_shortcut(target_exe: &Path) {
    let roaming = env::var("APPDATA").expect("APPDATA not set");
    let start_menu = PathBuf::from(roaming)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Grok CLI.lnk");

    // PowerShell script to create shortcut
    let script = format!(
        "$WS = New-Object -ComObject WScript.Shell; $SC = $WS.CreateShortcut('{}'); $SC.TargetPath = '{}'; $SC.Save()",
        start_menu.display(),
        target_exe.display()
    );

    let status = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .status()
        .expect("Failed to create shortcut");

    if status.success() {
        println!("Shortcut created at: {}", start_menu.display());
    } else {
        eprintln!("{}", "Failed to create Start Menu shortcut.".yellow());
    }
}

#[cfg(windows)]
#[cfg(windows)]
/// Copy agent preset TOML files from `.grok/agents/` (project source) into
/// `~/.grok-cli/agents/` (user-global agents directory).
///
/// Rules:
/// - Creates `~/.grok-cli/agents/` if it does not exist.
/// - Copies every `*.toml` found in `<root>/.grok/agents/`.
/// - **Does NOT overwrite** if the user already has a customised copy —
///   their edits are preserved.
/// - Prints a summary line for each file.
fn setup_agent_presets(root_dir: &Path) {
    let home_dir = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("setup_agent_presets: could not locate home directory");
            return;
        }
    };

    let agents_dst = home_dir.join(".grok-cli").join("agents");
    if let Err(e) = fs::create_dir_all(&agents_dst) {
        eprintln!(
            "setup_agent_presets: failed to create agents directory: {}",
            e
        );
        return;
    }

    // Shipped presets live in config/agents/ (committed to git, never user-specific).
    // .grok/agents/ is reserved for PROJECT-LOCAL overrides — don't ship anything there.
    let agents_src = root_dir.join("config").join("agents");
    if !agents_src.exists() {
        println!("No config/agents/ directory found in project root — skipping agent presets.");
        return;
    }

    let entries = match fs::read_dir(&agents_src) {
        Ok(e) => e,
        Err(e) => {
            eprintln!(
                "setup_agent_presets: failed to read agents directory: {}",
                e
            );
            return;
        }
    };

    let mut installed = 0u32;
    let mut skipped = 0u32;

    for entry in entries.flatten() {
        let src_path = entry.path();
        if !src_path.is_file() || src_path.extension().map(|e| e != "toml").unwrap_or(true) {
            continue;
        }

        let filename = match src_path.file_name() {
            Some(n) => n,
            None => continue,
        };
        let dst_path = agents_dst.join(filename);

        if dst_path.exists() {
            // Preserve user customisations.
            println!(
                "  ✓ Kept existing agent preset: ~/.grok-cli/agents/{}",
                filename.to_string_lossy()
            );
            skipped += 1;
        } else {
            match fs::copy(&src_path, &dst_path) {
                Ok(_) => {
                    println!(
                        "  → Installed agent preset: ~/.grok-cli/agents/{}",
                        filename.to_string_lossy()
                    );
                    installed += 1;
                }
                Err(e) => {
                    eprintln!(
                        "  ⚠ Failed to install {}: {}",
                        filename.to_string_lossy(),
                        e
                    );
                }
            }
        }
    }

    println!(
        "Agent presets: {} installed, {} preserved (user-customised).",
        installed, skipped
    );
    println!("  Location: {}", agents_dst.display());
    println!("  Presets: planner, coder, researcher, verifier");
    println!("  Customise any preset by editing its .toml file.");
    println!("  Add your own by creating a new .toml in that directory.");
}

#[cfg(windows)]
fn install_additional_files(root_dir: &Path, install_dir: &Path) {
    let base_install_dir = install_dir
        .parent()
        .expect("Failed to get parent directory");

    // Install LICENSE
    let license_src = root_dir.join("LICENSE");
    let license_dst = base_install_dir.join("LICENSE");
    if license_src.exists()
        && let Err(e) = fs::copy(&license_src, &license_dst)
    {
        eprintln!("Failed to copy LICENSE: {}", e);
    }

    // Install project_layout.md alongside the binary
    let layout_src = root_dir.join("project_layout.md");
    let layout_dst = base_install_dir.join("project_layout.md");
    if layout_src.exists()
        && let Err(e) = fs::copy(&layout_src, &layout_dst)
    {
        eprintln!("Failed to copy project_layout.md: {}", e);
    }

    // Create docs directory
    let docs_dir = base_install_dir.join("docs");
    if let Err(e) = fs::create_dir_all(&docs_dir) {
        eprintln!("Failed to create docs directory: {}", e);
        return;
    }

    // ── Core markdown docs (root level) ────────────────────────────────────
    let core_docs: &[(&str, &str)] = &[
        ("README.md", "README.md"),
        ("CONFIGURATION.md", "CONFIGURATION.md"),
        ("CHANGELOG.md", "CHANGELOG.md"),
        ("CONTRIBUTING.md", "CONTRIBUTING.md"),
        ("SETUP.md", "SETUP.md"),
        ("TROUBLESHOOTING.md", "TROUBLESHOOTING.md"),
        ("TESTING_TOOLS.md", "TESTING_TOOLS.md"),
        ("dataflow_map.md", "dataflow_map.md"),
    ];

    for (src_path, dst_name) in core_docs {
        let src = root_dir.join(src_path);
        let dst = docs_dir.join(dst_name);
        if src.exists()
            && let Err(e) = fs::copy(&src, &dst)
        {
            eprintln!("Failed to copy {src_path}: {e}");
        }
    }

    // ── Technical architecture docs (docs/) ─────────────────────────────────
    let arch_docs: &[(&str, &str)] = &[
        ("docs/rpl_architecture.md", "rpl_architecture.md"),
        ("docs/engine_architecture.md", "engine_architecture.md"),
        ("docs/REASONING_SYSTEMS.md", "REASONING_SYSTEMS.md"),
        ("docs/agent_docs.md", "agent_docs.md"),
    ];

    for (src_path, dst_name) in arch_docs {
        let src = root_dir.join(src_path);
        let dst = docs_dir.join(dst_name);
        if src.exists()
            && let Err(e) = fs::copy(&src, &dst)
        {
            eprintln!("Failed to copy {src_path}: {e}");
        }
    }

    // ── User-facing quick-reference docs (Doc/) ─────────────────────────────
    let user_docs: &[(&str, &str)] = &[
        ("Doc/QUICK_REFERENCE.md", "QUICK_REFERENCE.md"),
        ("Doc/CONFIG_QUICK_START.md", "CONFIG_QUICK_START.md"),
        ("Doc/SKILLS_QUICK_START.md", "SKILLS_QUICK_START.md"),
        ("Doc/REASONING_QUICK_START.md", "REASONING_QUICK_START.md"),
        (
            "Doc/EXTERNAL_ACCESS_QUICK_START.md",
            "EXTERNAL_ACCESS_QUICK_START.md",
        ),
        ("Doc/HOOKS_AND_EXTENSIONS.md", "HOOKS_AND_EXTENSIONS.md"),
        ("Doc/SECURITY.md", "SECURITY.md"),
        (
            "Doc/MAX_TOOL_LOOP_ITERATIONS.md",
            "MAX_TOOL_LOOP_ITERATIONS.md",
        ),
        ("Doc/extensions.md", "extensions.md"),
        ("Doc/SUBAGENTS.md", "SUBAGENTS.md"),
    ];

    for (src_path, dst_name) in user_docs {
        let src = root_dir.join(src_path);
        let dst = docs_dir.join(dst_name);
        if src.exists()
            && let Err(e) = fs::copy(&src, &dst)
        {
            eprintln!("Failed to copy {src_path}: {e}");
        }
    }

    // ── Scan entire Doc/ directory for any remaining .md files ──────────────
    let doc_dir = root_dir.join("Doc");
    if doc_dir.exists()
        && let Ok(entries) = fs::read_dir(&doc_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().is_some_and(|e| e == "md")
                && let Some(filename) = path.file_name()
            {
                // Skip files already explicitly listed above
                let dst = docs_dir.join(filename);
                if !dst.exists()
                    && let Err(e) = fs::copy(&path, &dst)
                {
                    eprintln!("Failed to copy {}: {e}", path.display());
                }
            }
        }
    }

    // ── Example skills ───────────────────────────────────────────────────────
    let skills_src = root_dir.join("examples").join("skills");
    let skills_dst = base_install_dir.join("examples").join("skills");
    if skills_src.exists()
        && let Err(e) = copy_dir_recursive(&skills_src, &skills_dst)
    {
        eprintln!("Failed to copy example skills: {e}");
    }

    println!("Documentation and examples installed successfully.");
    println!("  Docs location: {}", docs_dir.display());
}

#[cfg(windows)]
fn copy_dir_recursive(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}
