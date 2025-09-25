use anyhow::{Result, Context};
use std::path::PathBuf;
use std::fs;
use dirs;
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
    println!("🚢 Initializing Cargo Mate...");
    let mut config = crate::captain::config::ConfigManager::new()?;
    config.init_local()?;
    println!("✅ Local config created: .cg");
    println!("🔧 Setting up shell integration...");
    let shell = ShellIntegration::detect_shell()?;
    let rc_file = ShellIntegration::get_rc_file(&shell)?;
    if rc_file.exists() {
        let content = std::fs::read_to_string(&rc_file)?;
        if content.contains("# === Cargo Mate") {
            eprintln!("⚠️  Shell integration already installed");
        } else {
            ShellIntegration::add_shell_integration(&rc_file, &shell)?;
        }
    } else {
        ShellIntegration::add_shell_integration(&rc_file, &shell)?;
    }
    eprintln!("📁 Error logs will be stored in ~/.shipwreck/");
    println!();
    println!("🎉 Cargo Mate initialized successfully!");
    println!();
    println!(
        "⚡ {}", "Shell integration added. To activate immediately, run one of these:"
        .yellow()
    );
    println!("   {} {}", "source".green(), format!("{}", rc_file.display()) .cyan());
    println!("   {} {}", "cm".green(), "activate".cyan());
    println!("   {}", "Or restart your terminal".dimmed());
    println!();
    println!("📚 {}", "Available commands after activation:".yellow());
    println!("   {} - Run cargo through cargo-mate", "cargo".cyan());
    println!("   {} - Direct cargo-mate access", "cm".cyan());
    println!("   {} - Quick shortcut", "cg".cyan());
    println!();
    println!("💡 {}", "Tip: Run 'cm activate' anytime to activate integration".blue());
    Ok(())
}
pub fn handle_init() -> Result<()> {
    init_cargo_mate()
}