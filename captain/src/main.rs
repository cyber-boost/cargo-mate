// Cargo Mate - Rust On!
// A publishable binary that provides installation instructions and downloads required scripts

use std::env;
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚢 Cargo Mate - Rust On!");
    println!("==========================================");
    println!("ℹ️  🚢 Installing Cargo Mate (Source Protected)");

    let platform = detect_platform();
    println!("ℹ️  Detected platform: {}", platform);

    // For publishing, we download the installation scripts instead of embedding them
    println!("ℹ️  Downloading installation scripts...");

    let install_script_url = "https://get.cargo.do/mate";

    match download_and_run_installer(&install_script_url) {
        Ok(_) => {
            println!("✅ Installation completed successfully!");
        }
        Err(e) => {
            eprintln!("❌ Installation failed: {}", e);
            show_manual_installation();
        }
    }

    Ok(())
}

fn download_and_run_installer(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Use curl or wget to download the installer
    let temp_dir = std::env::temp_dir().join("cargo-mate-install");
    std::fs::create_dir_all(&temp_dir)?;
    let install_script_path = temp_dir.join("install.sh");

    let download_success = if command_exists("curl") {
        Command::new("curl")
            .args(&["-fsSL", url, "-o", &install_script_path.to_string_lossy()])
            .status()?
            .success()
    } else if command_exists("wget") {
        Command::new("wget")
            .args(&["-O", &install_script_path.to_string_lossy(), url])
            .status()?
            .success()
    } else {
        return Err("Neither curl nor wget found. Please install one of them.".into());
    };

    if !download_success {
        return Err("Failed to download installation script".into());
    }

    // Make executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&install_script_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&install_script_path, perms)?;
    }

    // Run the installer
    let status = Command::new("bash")
        .arg(&install_script_path)
        .status()?;

    if !status.success() {
        return Err(format!("Installation script failed with exit code: {}",
                          status.code().unwrap_or(-1)).into());
    }

    Ok(())
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
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
    eprintln!("🔧 Use one-click installer:");
    eprintln!("curl -fsSL https://get.cargo.do/mate | bash");
    eprintln!("");
    eprintln!("📦 The one-click installer will automatically install C compiler and cargo-mate.");
}