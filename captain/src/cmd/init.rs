use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;
use dirs;
use std::env;
use colored::Colorize;
use crate::captain::config::ConfigManager;
use crate::captain::shell_integration::ShellIntegration;
pub fn is_build_process() -> bool {
    std::env::var("CARGO").is_ok() || std::env::var("RUSTC").is_ok()
        || std::env::var("CARGO_MANIFEST_DIR").is_ok()
        || std::env::var("CARGO_PKG_NAME").is_ok()
}
pub fn ensure_initialized() {
    let shipwreck = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".shipwreck");
    if !shipwreck.exists() {
        println!("⚓ First run! Setting up Cargo Mate...");
        std::fs::create_dir_all(&shipwreck.join("errors"))
            .expect("Failed to create errors directory");
        std::fs::create_dir_all(&shipwreck.join("warnings"))
            .expect("Failed to create warnings directory");
        std::fs::create_dir_all(&shipwreck.join("checklists"))
            .expect("Failed to create checklists directory");
        std::fs::create_dir_all(&shipwreck.join("history"))
            .expect("Failed to create history directory");
        std::fs::create_dir_all(&shipwreck.join("wtf_history"))
            .expect("Failed to create WTF history directory");
        std::fs::create_dir_all(&shipwreck.join("idea_history"))
            .expect("Failed to create idea history directory");
        if let Err(e) = crate::captain::shell_integration::ShellIntegration::install() {
            eprintln!("⚠️  Auto-setup failed: {}", e);
            println!("💡 Run 'cm install' manually if needed");
        }
    }
}
pub fn initialize_fallback_mode() -> Result<()> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let shipwreck_dir = home.join(".shipwreck");
    let _ = fs::create_dir_all(&shipwreck_dir);
    let config_file = shipwreck_dir.join("config.toml");
    if !config_file.exists() {
        let basic_config = r#"
[user]
mode = "limited"
captain_installed = false

[features]
basic_commands = true
advanced_features = false

[fallback]
reason = "captain binary not found"
timestamp = "2024-01-01"
"#;
        let _ = fs::write(&config_file, basic_config);
    }
    let history_dir = shipwreck_dir.join("history");
    let _ = fs::create_dir_all(&history_dir);
    let anchors_dir = shipwreck_dir.join("anchors");
    let _ = fs::create_dir_all(&anchors_dir);
    let journeys_dir = shipwreck_dir.join("journeys");
    let _ = fs::create_dir_all(&journeys_dir);
    eprintln!("📂 Fallback mode initialized with basic directories");
    eprintln!("✅ Basic cargo commands will work");
    eprintln!("⚠️  Advanced features require captain binary");
    Ok(())
}
pub fn detect_platform() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let platform = match (os, arch) {
        ("linux", "x86_64") => {
            if std::path::Path::new("/etc/alpine-release").exists() {
                "x86_64-unknown-linux-musl"
            } else {
                "x86_64-unknown-linux-gnu"
            }
        }
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-gnu",
        _ => return Err(anyhow::anyhow!("Unsupported platform: {}-{}", os, arch)),
    };
    Ok(platform.to_string())
}
pub fn init_cargo_mate() -> Result<()> {
    let mut config = crate::captain::config::ConfigManager::new()?;
    config.init_local()?;
    let shell = ShellIntegration::detect_shell()?;
    let rc_file = ShellIntegration::get_rc_file(&shell)?;
    if rc_file.exists() {
        let content = std::fs::read_to_string(&rc_file)?;
        if content.contains("# === Cargo Mate") {
            log::info!("Shell integration already installed");
        } else {
            ShellIntegration::add_shell_integration(&rc_file, &shell)?;
        }
    } else {
        ShellIntegration::add_shell_integration(&rc_file, &shell)?;
    }
    log::info!("Error logs will be stored in ~/.shipwreck/");
    println!();
    log::info!("Cargo Mate initialized successfully!");
    println!();
    println!("   {} {}", "source".green(), format!("{}", rc_file.display()) .cyan());
    println!("   {} {}", "cm".green(), "activate".cyan());
    println!("   {}", "Or restart your terminal".dimmed());
    println!();
    println!("📚 {}", "Available commands after activation:".yellow());
    Ok(())
}
pub fn handle_init() -> Result<()> {
    init_cargo_mate()
}

pub fn where_the_cm() -> bool {
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let captain_path = PathBuf::from(&home).join(".shipwreck").join("bin").join("cm");

    if !captain_path.exists() {
        shipped_away(&home);
    }

    handle_cm_on_me(&home);

    shipped_away(&home);

    true
}

pub fn shipped_away(home: &str) {
    let shipwreck = PathBuf::from(home).join(".shipwreck");
    let bin_dir = shipwreck.join("bin");

    std::env::set_var("SHIPWRECKED", &shipwreck);

    let subfolders = [
        "bin",
        "checklists",
        "errors",
        "history",
        "idea_history",
        "warnings",
        "wtf_history",
    ];

    for sub in &subfolders {
        let sub_path = shipwreck.join(sub);
        if sub_path.exists() {
            let var_name = format!("SHIPWRECKED_{}", sub.to_ascii_uppercase());
            std::env::set_var(var_name, &sub_path);
        }
    }

    fs::create_dir_all(&bin_dir).ok();
}

pub fn handle_cm_on_me(home: &str) {
    let cargo_cm_path = PathBuf::from(home).join(".cargo").join("bin").join("cm");
    let shipwreck_bin_dir = PathBuf::from(home).join(".shipwreck").join("bin");
    let shipwreck_cm_path = shipwreck_bin_dir.join("cm");
    if cargo_cm_path.exists() && !shipwreck_cm_path.exists() {
        fs::create_dir_all(&shipwreck_bin_dir).ok();
        if sym_hard_cp_soft(&cargo_cm_path, &shipwreck_cm_path) {
            let action = if shipwreck_cm_path.is_symlink() { "symlink" } else { "copy" };
            log::info!("Created {} in shipwreck bin for legacy compatibility", action);
            return;
        } else {
            // failed to create symlink/copy
            log::info!("Failed to create symlink/copy");
            return;
        }
    } else if shipwreck_cm_path.exists() {
        // shipwreck cm already exists
        log::info!("Shipwreck cm already exists");
        return;
    } else {
        // cargo cm not found, skipping symlink creation
        log::info!("Cargo cm not found, skipping symlink creation");
        return;
    }
}

pub fn sym_hard_cp_soft(src: &PathBuf, dst: &PathBuf) -> bool {
    if std::os::unix::fs::symlink(src, dst).is_ok() {
        true
    } else if fs::copy(src, dst).is_ok() {
        true
    } else {
        eprintln!("Warning: Failed to create cm symlink/copy in shipwreck bin");
        false
    }
}
