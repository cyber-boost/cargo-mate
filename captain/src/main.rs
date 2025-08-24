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
    let temp_dir = tempfile::tempdir()?;
    let install_dir = temp_dir.path();
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
    copy_protected_binaries(install_dir, &platform)?;
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

fn copy_protected_binaries(install_dir: &Path, platform: &str) -> anyhow::Result<()> {
    let platform_dir = install_dir.join(platform);
    fs::create_dir_all(&platform_dir)?;
    let binary_names = match platform {
        "linux" => vec![
            ("cargo-mate-linux-x86_64.protected", "linux/cargo-mate-linux-x86_64.protected"),
            ("cargo-mate-linux-aarch64.protected", "linux/cargo-mate-linux-aarch64.protected"),
        ],
        "macos" => vec![
            ("cargo-mate-macos-x86_64.protected", "macos/cargo-mate-macos-x86_64.protected"),
            ("cargo-mate-macos-aarch64.protected", "macos/cargo-mate-macos-aarch64.protected"),
        ],
        "windows" => vec![
            ("cargo-mate-windows-x86_64.exe.protected", "windows/cargo-mate-windows-x86_64.exe.protected"),
        ],
        _ => return Err(anyhow::anyhow!("Unsupported platform: {}", platform)),
    };
    for (dest_name, source_path) in binary_names {
        let dest_path = platform_dir.join(dest_name);
        let current_dir = env::current_dir()?;
        let source_file = current_dir.join(source_path);
        if source_file.exists() {
            fs::copy(&source_file, &dest_path)?;
        } else {
            fs::write(&dest_path, b"PLACEHOLDER: Protected binary would be here")?;
        }
    }
    Ok(())
}
fn show_manual_installation() {
    eprintln!("🔧 Manual Installation Instructions:");
    eprintln!("curl -sSf https://cargo.do/install.sh | bash");
}