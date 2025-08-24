// Cargo Mate - Rust On!

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

// Embedded install scripts - these are compiled into the binary
const INSTALL_SCRIPT: &str = include_str!("../sh/install.sh");
const WRAPPER_LINUX: &str = include_str!("../sh/wrapper-linux.sh");
const WRAPPER_MACOS: &str = include_str!("../sh/wrapper-macos.sh");
const WRAPPER_WINDOWS_BAT: &str = include_str!("../sh/wrapper-windows.bat");
const WRAPPER_WINDOWS_PS1: &str = include_str!("../sh/wrapper-windows.ps1");

fn main() -> anyhow::Result<()> {
    println!("🚢 Cargo Mate - Rust On!");
    println!("==========================================");
    let temp_dir = std::env::temp_dir().join("cargo-mate-install");
    std::fs::create_dir_all(&temp_dir)?;
    let install_dir = &temp_dir;
    let install_script_path = install_dir.join("install.sh");
    fs::write(&install_script_path, INSTALL_SCRIPT)?;

    let mut perms = fs::metadata(&install_script_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&install_script_path, perms)?;

    let platform = detect_platform();
    let sh_dir = install_dir.join("sh");
    fs::create_dir_all(&sh_dir)?;

    match platform.as_str() {
        "linux" => {
            let wrapper_path = sh_dir.join("wrapper-linux.sh");
            fs::write(&wrapper_path, WRAPPER_LINUX)?;
            let mut perms = fs::metadata(&wrapper_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&wrapper_path, perms)?;
        }
        "macos" => {
            let wrapper_path = sh_dir.join("wrapper-macos.sh");
            fs::write(&wrapper_path, WRAPPER_MACOS)?;
            let mut perms = fs::metadata(&wrapper_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&wrapper_path, perms)?;
        }
        "windows" => {
            fs::write(sh_dir.join("wrapper-windows.bat"), WRAPPER_WINDOWS_BAT)?;
            fs::write(sh_dir.join("wrapper-windows.ps1"), WRAPPER_WINDOWS_PS1)?;
        }
        _ => {
            return Err(anyhow::anyhow!("Unsupported platform: {}", platform));
        }
    }
    let status = Command::new("bash")
        .arg(&install_script_path)
        .current_dir(install_dir)
        .status()?;

    if status.success() {
        println!("✅ Installation completed successfully!");
    } else {
        eprintln!("❌ Installation failed with exit code: {}", status.code().unwrap_or(-1));
        show_manual_installation();
    }

    Ok(())
}

fn detect_platform() -> String {
    match env::consts::OS {
        "linux" => "linux".to_string(),
        "macos" => "macos".to_string(),
        "windows" => "windows".to_string(),
        _ => env::consts::OS.to_string(),
    }
}


fn show_manual_installation() {
    eprintln!("🔧 Manual Installation Instructions:");
    eprintln!("curl -sSf https://cargo.do/install.sh | bash");
}