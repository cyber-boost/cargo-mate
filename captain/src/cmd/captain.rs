use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::Write;
use tempfile::NamedTempFile;
use std::env;
pub fn auto_install_captain() -> Result<()> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let platform = match os {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => return Err(anyhow::anyhow!("Unsupported operating system: {}", os)),
    };
    let architecture = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return Err(anyhow::anyhow!("Unsupported architecture: {}", arch)),
    };
    let download_url = format!(
        "https://cargo.do/captain/captain-{}-{}.tar.gz", platform, architecture
    );
    let temp_dir = tempfile::tempdir()?;
    let archive_path = temp_dir.path().join("captain.tar.gz");
    log::info!("📥 Downloading captain binary from {}...", download_url);
    let response = reqwest::blocking::get(&download_url)?;
    let content = response.bytes()?;
    let mut file = fs::File::create(&archive_path)?;
    file.write_all(&content)?;
    log::info!("📦 Extracting captain binary...");
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .current_dir(temp_dir.path())
        .output()?;
    if !output.status.success() {
        return Err(
            anyhow::anyhow!(
                "Failed to extract captain archive: {}", String::from_utf8_lossy(& output
                .stderr)
            ),
        );
    }
    let captain_binary = temp_dir.path().join("captain");
    let captain_enc = temp_dir.path().join("captain.enc");
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    let shipwreck_dir = home_dir.join(".shipwreck");
    let shipwreck_bin = shipwreck_dir.join("bin");
    fs::create_dir_all(&shipwreck_bin)?;
    let source_binary = if captain_binary.exists() {
        eprintln!("✅ Found self-contained captain binary");
        captain_binary.clone()
    } else if captain_enc.exists() {
        eprintln!("ℹ️ Found legacy captain.enc binary (requires PROTECT_KEY)");
        captain_enc
    } else {
        return Err(anyhow::anyhow!("No captain binary found in archive"));
    };
    let captain_dest = if source_binary == captain_binary {
        shipwreck_bin.join("captain")
    } else {
        shipwreck_bin.join("captain.protected")
    };
    fs::copy(&source_binary, &captain_dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&captain_dest)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&captain_dest, perms)?;
    }
    if source_binary == captain_binary {
        let symlink_path = shipwreck_bin.join("captain.protected");
        if symlink_path.exists() {
            fs::remove_file(&symlink_path).ok();
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&captain_dest, &symlink_path).ok();
        }
    }
    log::info!("✅ Captain binary installed to {}", captain_dest.display());
    Ok(())
}