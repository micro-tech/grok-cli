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

    // 6. Update PATH
    println!("{}", "Updating PATH environment variable...".cyan());
    update_path(&install_dir);

    // 7. Create Start Menu Shortcut
    println!("{}", "Creating Start Menu shortcut...".cyan());
    create_shortcut(&target_exe);

    // 8. Setup Configuration
    println!("{}", "Setting up configuration...".cyan());
    setup_config();

    // 9. Setup Global Context
    println!("{}", "Setting up global context...".cyan());
    setup_context(&root_dir);

    println!("\n{}", "Installation Complete!".green().bold());
    println!("Please restart your terminal to use the 'grok' command.");
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
fn setup_config() {
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
