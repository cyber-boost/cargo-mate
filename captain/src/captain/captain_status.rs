use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use colored::*;
use tempfile::tempdir;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CaptainStatus {
    NotInstalled,
    Installed {
        path: String,
        installed_at: String,
        version: Option<String>,
        last_verified: String,
    },
    Corrupted { path: String, reason: String },
}
pub struct SimpleCaptainStatus {
    pub is_installed: bool,
    pub binary_path: Option<PathBuf>,
    pub version: Option<String>,
}
static CAPTAIN_STATUS: std::sync::Mutex<Option<CaptainStatus>> = std::sync::Mutex::new(
    None,
);
fn get_status_file_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let shipwreck_dir = PathBuf::from(&home).join(".shipwreck");
    fs::create_dir_all(&shipwreck_dir)?;
    Ok(shipwreck_dir.join("captain_status.json"))
}
fn load_captain_status() -> Option<CaptainStatus> {
    let status_file = match get_status_file_path() {
        Ok(path) => path,
        Err(_) => return None,
    };
    if !status_file.exists() {
        return None;
    }
    match fs::read_to_string(&status_file) {
        Ok(content) => {
            match serde_json::from_str::<CaptainStatus>(&content) {
                Ok(status) => Some(status),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}
fn save_captain_status(status: &CaptainStatus) -> Result<()> {
    let status_file = get_status_file_path()?;
    let content = serde_json::to_string_pretty(status)?;
    fs::write(&status_file, content)?;
    Ok(())
}
fn verify_captain_binary(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Binary not found: {}", path));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
        let permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            return Err(format!("Binary is not executable: {}", path));
        }
    }
    Ok(())
}
pub fn get_captain_status() -> SimpleCaptainStatus {
    let binary_path = find_captain_binary();
    SimpleCaptainStatus {
        is_installed: binary_path.is_some(),
        binary_path,
        version: None,
    }
}
pub fn auto_download_captain() -> Result<PathBuf> {
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
        "https://get.cargo.do/captain/captain-{}-{}.tar.gz", platform, architecture
    );
    println!("📥 {}", format!("Downloading from: {}", download_url) .bright_black());
    let temp_dir = tempdir()?;
    let archive_path = temp_dir.path().join("captain.tar.gz");
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .context("Failed to download captain binary")?;
    if !response.status().is_success() {
        return Err(
            anyhow::anyhow!("Download failed with status: {}", response.status()),
        );
    }
    let content = response.bytes()?;
    let mut file = fs::File::create(&archive_path)?;
    file.write_all(&content)?;
    println!("📦 {}", "Extracting captain binary...".bright_blue());
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .current_dir(temp_dir.path())
        .output()
        .context("Failed to extract captain archive")?;
    if !output.status.success() {
        return Err(
            anyhow::anyhow!(
                "Extraction failed: {}", String::from_utf8_lossy(& output.stderr)
            ),
        );
    }
    let captain_binary = temp_dir.path().join("captain");
    if !captain_binary.exists() {
        let captain_protected = temp_dir.path().join("captain.protected");
        let captain_exe = temp_dir.path().join("captain.exe");
        if captain_protected.exists() {
            println!(
                "🔐 {}", "Found protected captain binary (requires protection key)"
                .yellow()
            );
            return Err(
                anyhow::anyhow!(
                    "Protected binary found - please install manually with: cm captain install"
                ),
            );
        } else if captain_exe.exists() {
            return Err(anyhow::anyhow!("Windows binary found on non-Windows system"));
        } else {
            return Err(anyhow::anyhow!("No captain binary found in archive"));
        }
    }
    let home_dir = dirs::home_dir().context("Could not determine home directory")?;
    let shipwreck_dir = home_dir.join(".shipwreck");
    let shipwreck_bin = shipwreck_dir.join("bin");
    fs::create_dir_all(&shipwreck_bin)?;
    let captain_dest = shipwreck_bin.join("captain");
    fs::copy(&captain_binary, &captain_dest)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&captain_dest)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&captain_dest, perms)?;
    }
    Ok(captain_dest)
}
pub fn find_captain_binary() -> Option<PathBuf> {
    let possible_paths = vec![
        dirs::home_dir().map(| p | p.join(".shipwreck").join("bin").join("captain")),
        dirs::home_dir().map(| p | p.join(".cargo").join("bin").join("captain")),
        Some(PathBuf::from("/usr/local/bin/captain")),
        Some(PathBuf::from("/usr/bin/captain")), dirs::home_dir().map(| p | p
        .join(".shipwreck").join("bin").join("captain.protected")), dirs::home_dir()
        .map(| p | p.join(".cargo").join("bin").join("captain.protected")),
        Some(PathBuf::from("/usr/local/bin/captain.protected")),
        Some(PathBuf::from("/usr/bin/captain.protected")),
    ];
    for path in possible_paths.into_iter().flatten() {
        if path.exists() {
            return Some(path);
        }
    }
    None
}
pub fn is_captain_available() -> bool {
    if let Some(path) = find_captain_binary() {
        if path.to_string_lossy().ends_with("/captain")
            || path.file_name().map_or(false, |f| f == "captain")
        {
            return true;
        }
        match verify_captain_binary(&path.to_string_lossy()) {
            Ok(_) => true,
            Err(_) => if std::env::var("PROTECT_KEY").is_ok() { true } else { false }
        }
    } else {
        false
    }
}
fn get_cached_status() -> CaptainStatus {
    if let Ok(cache) = CAPTAIN_STATUS.lock() {
        if let Some(status) = &*cache {
            return status.clone();
        }
    }
    if let Some(captain_path) = find_captain_binary() {
        let captain_path_str = captain_path.to_string_lossy().to_string();
        match verify_captain_binary(&captain_path_str) {
            Ok(_) => {
                mark_captain_installed(&captain_path_str).ok();
                CaptainStatus::Installed {
                    path: captain_path_str,
                    installed_at: chrono::Utc::now().to_rfc3339(),
                    version: None,
                    last_verified: chrono::Utc::now().to_rfc3339(),
                }
            }
            Err(reason) => {
                mark_captain_corrupted(&captain_path_str, &reason).ok();
                CaptainStatus::Corrupted {
                    path: captain_path_str,
                    reason,
                }
            }
        }
    } else {
        CaptainStatus::NotInstalled
    }
}
pub fn get_captain_path() -> Option<String> {
    match get_cached_status() {
        CaptainStatus::Installed { path, .. } => Some(path),
        _ => find_captain_binary().map(|p| p.to_string_lossy().to_string()),
    }
}
pub fn mark_captain_installed(path: &str) -> Result<()> {
    if let Ok(mut cache) = CAPTAIN_STATUS.lock() {
        *cache = Some(CaptainStatus::Installed {
            path: path.to_string(),
            installed_at: chrono::Utc::now().to_rfc3339(),
            version: None,
            last_verified: chrono::Utc::now().to_rfc3339(),
        });
    }
    Ok(())
}
pub fn mark_captain_corrupted(path: &str, reason: &str) -> Result<()> {
    if let Ok(mut cache) = CAPTAIN_STATUS.lock() {
        *cache = Some(CaptainStatus::Corrupted {
            path: path.to_string(),
            reason: reason.to_string(),
        });
    }
    Ok(())
}
pub fn clear_captain_status() -> Result<()> {
    let status_file = get_status_file_path()?;
    if status_file.exists() {
        fs::remove_file(&status_file)?;
    }
    if let Ok(mut cache) = CAPTAIN_STATUS.lock() {
        *cache = None;
    }
    Ok(())
}
pub fn refresh_captain_status() -> Result<CaptainStatus> {
    if let Ok(mut cache) = CAPTAIN_STATUS.lock() {
        *cache = None;
    }
    if let Some(captain_path) = find_captain_binary() {
        match verify_captain_binary(&captain_path.to_string_lossy()) {
            Ok(_) => {
                mark_captain_installed(&captain_path.to_string_lossy())?;
                Ok(CaptainStatus::Installed {
                    path: captain_path.to_string_lossy().to_string(),
                    installed_at: chrono::Utc::now().to_rfc3339(),
                    version: None,
                    last_verified: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(reason) => {
                mark_captain_corrupted(
                    &captain_path.to_string_lossy().to_string(),
                    &reason,
                )?;
                Ok(CaptainStatus::Corrupted {
                    path: captain_path.to_string_lossy().to_string(),
                    reason,
                })
            }
        }
    } else {
        clear_captain_status()?;
        Ok(CaptainStatus::NotInstalled)
    }
}
pub fn get_captain_status_info() -> String {
    match get_cached_status() {
        CaptainStatus::NotInstalled => {
            format!(
                "Captain Status: Not Installed\nCaptain Paths Checked: {} locations",
                find_captain_binary().map_or("0".to_string(), | _ | "multiple"
                .to_string())
            )
        }
        CaptainStatus::Installed { path, installed_at, version, last_verified } => {
            format!(
                "Captain Status: Installed ✅\nPath: {}\nInstalled At: {}\nLast Verified: {}\nVersion: {}",
                path, installed_at, last_verified, version.as_ref().unwrap_or(& "Unknown"
                .to_string())
            )
        }
        CaptainStatus::Corrupted { path, reason } => {
            format!(
                "Captain Status: Corrupted ❌\nPath: {}\nReason: {}\nSuggestion: Reinstall captain",
                path, reason
            )
        }
    }
}