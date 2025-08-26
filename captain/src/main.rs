// Cargo Mate - Rust On!
// A publishable binary that provides installation instructions and downloads required scripts

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    // Check for help flag first
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        show_installation_methods();
        return;
    }

    println!("🚢 Cargo Mate Installer v{}", VERSION);
    println!("{}", "=".repeat(50));

    match install_cargo_mate() {
        Ok(install_path) => {
            println!("✅ Cargo Mate installed successfully to: {}", install_path.display());
            println!();
            println!("🚀 Run 'cm --help' to get started!");
            println!("💡 Or use 'cargo <command>' to use cargo through Cargo Mate");
        }
        Err(e) => {
            eprintln!("❌ Modern installation failed: {}", e);
            eprintln!("🔄 Falling back to script-based installation...");

            // Fallback to script-based installation
            let install_script_url = "https://get.cargo.do/mate";
            match download_and_run_installer(&install_script_url) {
                Ok(_) => {
                    println!("✅ Installation completed successfully via fallback method!");
                }
                Err(fallback_err) => {
                    eprintln!("❌ Fallback installation also failed: {}", fallback_err);
                    eprintln!();
                    show_installation_methods();
                    std::process::exit(1);
                }
            }
        }
    }
}

fn install_cargo_mate() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let (platform, arch) = detect_platform();

    println!("📍 Detected platform: {}-{}", platform, arch);

    // Determine the correct binary URL
    let binary_name = match (platform.as_str(), arch.as_str()) {
        ("linux", "x86_64") => {
            if is_musl_system() {
                "cargo-mate-static-linux-x86_64"
            } else {
                "cargo-mate-linux-x86_64"
            }
        },
        ("linux", "aarch64") => "cargo-mate-linux-aarch64",
        ("macos", "x86_64") => "cargo-mate-macos-x86_64",
        ("macos", "aarch64") => "cargo-mate-macos-aarch64",
        ("windows", _) => "cargo-mate-windows-x86_64",
        _ => return Err(format!("Unsupported platform: {}-{}", platform, arch).into()),
    };

    let download_url = format!("https://get.cargo.do/dist/{}.tar.gz", binary_name);
    println!("📦 Downloading from: {}", download_url);

    // Get cargo bin directory
    let cargo_home = env::var("CARGO_HOME")
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .expect("Could not find home directory")
                .join(".cargo")
                .to_string_lossy()
                .to_string()
        });

    let bin_dir = PathBuf::from(cargo_home).join("bin");
    fs::create_dir_all(&bin_dir)?;

    let target_path = bin_dir.join(if platform == "windows" { "cm.exe" } else { "cm" });

    // Download the binary
    download_and_extract(&download_url, &target_path)?;

    // Create cargo wrapper symlink/alias
    create_cargo_wrapper(&bin_dir)?;

    Ok(target_path)
}

fn download_and_extract(url: &str, target: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    let temp_dir = env::temp_dir().join("cargo-mate-install");
    fs::create_dir_all(&temp_dir)?;
    let temp_archive = temp_dir.join("cargo-mate.tar.gz");

    // Download
    print!("⬇️  Downloading package... ");
    std::io::stdout().flush()?;

    let download_success = if command_exists("curl") {
        Command::new("curl")
            .args(&["-fsSL", url, "-o"])
            .arg(&temp_archive)
            .status()?
            .success()
    } else if command_exists("wget") {
        Command::new("wget")
            .args(&["-q", "-O"])
            .arg(&temp_archive)
            .arg(url)
            .status()?
            .success()
    } else {
        return Err("Neither curl nor wget found. Please install one of them.".into());
    };

    if !download_success {
        return Err("Failed to download package".into());
    }
    println!("done!");

    // Extract
    print!("📂 Extracting... ");
    std::io::stdout().flush()?;

    let extract_success = Command::new("tar")
        .args(&["-xzf"])
        .arg(&temp_archive)
        .current_dir(&temp_dir)
        .status()?
        .success();

    if !extract_success {
        return Err("Failed to extract archive".into());
    }
    println!("done!");

    // Look for install.sh script first (easy install)
    let entries = fs::read_dir(&temp_dir)?;
    for entry in entries {
        let path = entry?.path();
        if path.is_file() && path.file_name().map_or(false, |n| n.to_string_lossy() == "install.sh") {
            println!("🔧 Found install.sh - running easy installer...");

            // Make executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&path, perms)?;
            }

            // Run the install script
            let install_result = Command::new("bash")
                .arg(&path)
                .current_dir(&temp_dir)
                .status()?;

            if install_result.success() {
                println!("✅ Easy install completed successfully!");
                // Clean up
                let _ = fs::remove_dir_all(&temp_dir);
                return Ok(());
            } else {
                println!("⚠️  install.sh failed, falling back to manual binary installation...");
            }
        }
    }

    // Fallback: Find the binary in extracted files (if install.sh didn't work)
    println!("🔍 Looking for binary files...");
    let entries = fs::read_dir(&temp_dir)?;
    for entry in entries {
        let path = entry?.path();
        if path.is_file() && path.file_name().map_or(false, |n| n.to_string_lossy().starts_with("cm")) {
            println!("📦 Installing binary manually...");
            fs::copy(&path, target)?;

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(target)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(target, perms)?;
            }

            // Clean up
            let _ = fs::remove_dir_all(&temp_dir);
            return Ok(());
        }
    }

    Err("Neither install.sh nor binary found in archive".into())
}

fn create_cargo_wrapper(_bin_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // The install.sh script handles the full installation including shell integration
    // This function is now mainly for future shell integration features
    println!("🔧 Shell integration handled by install.sh");
    println!("   Run 'cm --help' to see available commands");
    Ok(())
}

fn download_and_run_installer(url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Use curl or wget to download the installer
    let temp_dir = std::env::temp_dir().join("cargo-mate-install");
    std::fs::create_dir_all(&temp_dir)?;
    let install_script_path = temp_dir.join("install.sh");

    print!("⬇️  Downloading installation script... ");
    std::io::stdout().flush()?;

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
    println!("done!");

    // Make executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&install_script_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&install_script_path, perms)?;
    }

    // Run the installer
    print!("⚙️  Running installer... ");
    std::io::stdout().flush()?;

    let status = Command::new("bash")
        .arg(&install_script_path)
        .status()?;

    if !status.success() {
        return Err(format!("Installation script failed with exit code: {}",
                          status.code().unwrap_or(-1)).into());
    }
    println!("done!");

    Ok(())
}

fn detect_platform() -> (String, String) {
    let os = match env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => env::consts::OS,
    };

    let arch = match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => env::consts::ARCH,
    };

    (os.to_string(), arch.to_string())
}

fn is_musl_system() -> bool {
    // Check if system uses musl (Alpine Linux, etc.)
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        return content.contains("Alpine") || content.contains("musl");
    }

    // Check ldd version
    if let Ok(output) = std::process::Command::new("ldd")
        .arg("--version")
        .output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        return stdout.contains("musl");
    }

    false
}

fn command_exists(cmd: &str) -> bool {
    std::process::Command::new("which")
        .arg(cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}


fn show_installation_methods() {
    println!("📦 Installation Methods:");
    println!();
    println!("1. Using cm install (fastest, recommended):");
    println!("   cm install");
    println!("   ✅ Installs Cargo Mate instantly using the cm CLI");
    println!();
    println!("2. Using cargo-binstall:");
    println!("   cargo binstall cargo-mate");
    println!();
    println!("3. Quick & Dirty Installer (Works Everywhere... in Theory):");
    println!("   curl -sSL https://get.cargo.do/mate | bash");
    println!("   ✅ No compilation needed, auto-detects platform, handles all dependencies");
    println!();
    println!("4. Control & Freaky (Direct Download - Choose Your Platform and Speed):");
    println!("   # Linux x86_64");
    println!("   wget https://get.cargo.do/linux-x86-64.tar.gz");
    println!("   tar -xzf linux-x86-64.tar.gz && ./install.sh");
    println!();
    println!("   # Linux ARM64");
    println!("   wget https://get.cargo.do/linux-arm64.tar.gz");
    println!("   tar -xzf linux-arm64.tar.gz && ./install.sh");
    println!();
    println!("   # macOS Intel");
    println!("   wget https://get.cargo.do/macos-x86-64.tar.gz");
    println!("   tar -xzf macos-x86-64.tar.gz && ./install.sh");
    println!();
    println!("   # macOS Apple Silicon");
    println!("   wget https://get.cargo.do/macos-arm64.tar.gz");
    println!("   tar -xzf macos-arm64.tar.gz && ./install.sh");
    println!();
    println!("   # Windows");
    println!("   wget https://get.cargo.do/windows-x86-64.tar.gz");
    println!("   tar -xzf windows-x86-64.tar.gz");
    println!("   # Run install.ps1 in PowerShell");
    println!();
    println!("For more information: https://cargo.do");
}

// Minimal dependencies for the installer
mod dirs {
    use std::env;
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .ok()
            .map(PathBuf::from)
    }
}