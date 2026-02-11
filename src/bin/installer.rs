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

fn main() {
    println!("{}", "Grok CLI Installer for Windows 11".green().bold());
    println!("=======================================");

    if !cfg!(windows) {
        eprintln!("{}", "This installer is designed for Windows only.".red());
        std::process::exit(1);
    }

    #[cfg(windows)]
    install_windows();
}

#[cfg(windows)]
fn install_windows() {
    let root_dir = find_project_root().expect("Failed to find project root (Cargo.toml not found)");

    // 0. Check for old Cargo installation
    check_and_remove_old_cargo_install();

    // 1. Build the release binary
    println!("{}", "Building release binary...".cyan());
    let status = Command::new("cargo")
        .current_dir(&root_dir)
        .args(["build", "--release", "--bin", "grok"])
        .status()
        .expect("Failed to execute cargo build");

    if !status.success() {
        eprintln!("{}", "Build failed. Aborting installation.".red());
        std::process::exit(1);
    }

    // 2. Define paths
    let local_app_data = env::var("LOCALAPPDATA").expect("LOCALAPPDATA not set");
    let install_dir = PathBuf::from(&local_app_data).join("grok-cli").join("bin");
    let exe_name = "grok.exe";
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
                "Please make sure 'grok.exe' is not currently running.".yellow()
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

    println!("\n{}", "Installation Complete!".green().bold());
    println!("Please restart your terminal to use the 'grok' command.");
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
fn check_and_remove_old_cargo_install() {
    if let Some(home_dir) = dirs::home_dir() {
        let cargo_grok = home_dir.join(".cargo").join("bin").join("grok.exe");

        if cargo_grok.exists() {
            println!("\n{}", "Old Cargo installation detected!".yellow().bold());
            println!("Found old version at: {}", cargo_grok.display());

            // Try to get version
            if let Ok(output) = Command::new(&cargo_grok).arg("--version").output() {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    println!("Old version: {}", version.trim());
                }
            }

            print!(
                "\n{}",
                "Do you want to remove the old installation? (yes/no): ".yellow()
            );
            io::stdout().flush().unwrap();

            let mut response = String::new();
            io::stdin().read_line(&mut response).unwrap();

            if response.trim().eq_ignore_ascii_case("yes") {
                match fs::remove_file(&cargo_grok) {
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
                            "  Please close all running grok instances and try again.".yellow()
                        );
                        eprintln!("{}", "  Or manually delete: ".yellow());
                        eprintln!("  {}", cargo_grok.display());
                    }
                }
            } else {
                println!(
                    "{}",
                    "Skipping removal. You may have version conflicts.".yellow()
                );
                println!("{}", "  To remove later, delete: ".cyan());
                println!("  {}", cargo_grok.display());
            }
            println!();
        }
    }
}

#[cfg(windows)]
fn setup_context(root_dir: &Path) {
    if let Some(home_dir) = dirs::home_dir() {
        let grok_dir = home_dir.join(".grok");
        if !grok_dir.exists() {
            if let Err(e) = fs::create_dir_all(&grok_dir) {
                eprintln!("Failed to create .grok directory: {}", e);
                return;
            }
        }

        let source_context = root_dir.join("context.md");
        let target_context = grok_dir.join("context.md");

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

    // Copy example config if it exists
    let example_config_src = root_dir.join("config.example.toml");
    let example_config_dst = config_dir.join("config.example.toml");
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

    let config_file = config_dir.join("config.toml");
    if !config_file.exists() {
        println!("Configuration file not found at: {}", config_file.display());
        print!("Do you want to set up your Grok API Key now? [Y/n]: ");
        io::stdout().flush().unwrap_or_default();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let input = input.trim().to_lowercase();
            if input != "n" && input != "no" {
                print!("Enter your X API Key: ");
                io::stdout().flush().unwrap_or_default();
                let mut key = String::new();
                if io::stdin().read_line(&mut key).is_ok() {
                    let key = key.trim();
                    if !key.is_empty() {
                        let config_content = format!(
                            r#"# Grok CLI Configuration

# X API Key
api_key = "{}"

# Default Model
default_model = "grok-3"

# ACP Configuration
[acp]
max_tool_loop_iterations = 25

# Network Configuration
[network]
starlink_optimizations = true
health_monitoring = true
"#,
                            key
                        );
                        if let Err(e) = fs::write(&config_file, config_content) {
                            eprintln!("Failed to write config file: {}", e);
                        } else {
                            println!("Configuration saved to {}", config_file.display());
                        }
                    } else {
                        println!("Skipping API key setup (empty key provided).");
                    }
                }
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
fn install_additional_files(root_dir: &Path, install_dir: &Path) {
    let base_install_dir = install_dir
        .parent()
        .expect("Failed to get parent directory");

    // Install LICENSE
    let license_src = root_dir.join("LICENSE");
    let license_dst = base_install_dir.join("LICENSE");
    if license_src.exists() {
        if let Err(e) = fs::copy(&license_src, &license_dst) {
            eprintln!("Failed to copy LICENSE: {}", e);
        }
    }

    // Create docs directory
    let docs_dir = base_install_dir.join("docs");
    if let Err(e) = fs::create_dir_all(&docs_dir) {
        eprintln!("Failed to create docs directory: {}", e);
        return;
    }

    // Install core documentation files
    let core_docs = vec![
        ("README.md", "README.md"),
        ("CONFIGURATION.md", "CONFIGURATION.md"),
        ("CHANGELOG.md", "CHANGELOG.md"),
        (
            "Doc/MAX_TOOL_LOOP_ITERATIONS.md",
            "MAX_TOOL_LOOP_ITERATIONS.md",
        ),
    ];

    for (src_path, dst_name) in core_docs {
        let src = root_dir.join(src_path);
        let dst = docs_dir.join(dst_name);
        if src.exists() {
            if let Err(e) = fs::copy(&src, &dst) {
                eprintln!("Failed to copy {}: {}", src_path, e);
            }
        }
    }

    // Install Doc/docs/ files
    let doc_docs_dir = root_dir.join("Doc").join("docs");
    if doc_docs_dir.exists() {
        if let Ok(entries) = fs::read_dir(&doc_docs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "md" {
                            if let Some(filename) = path.file_name() {
                                let dst = docs_dir.join(filename);
                                if let Err(e) = fs::copy(&path, &dst) {
                                    eprintln!("Failed to copy {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Install example skills
    let skills_src = root_dir.join("examples").join("skills");
    let skills_dst = base_install_dir.join("examples").join("skills");
    if skills_src.exists() {
        if let Err(e) = copy_dir_recursive(&skills_src, &skills_dst) {
            eprintln!("Failed to copy example skills: {}", e);
        }
    }

    println!("Documentation and examples installed successfully.");
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
