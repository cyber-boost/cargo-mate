use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
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
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.len() < 1000 {
                return Err(
                    "Captain binary is too small (possibly corrupted)".to_string(),
                );
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = metadata.permissions().mode();
                if mode & 0o111 == 0 {
                    return Err("Captain binary is not executable".to_string());
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("Cannot access captain binary: {}", e)),
    }
}
pub fn find_captain_binary() -> Option<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let user = std::env::var("USER").unwrap_or_else(|_| "user".to_string());
    let captain_paths = [
        format!("{}/.shipwreck/bin/captain", home),
        "/root/.shipwreck/bin/captain".to_string(),
        "/usr/local/bin/captain".to_string(),
        "/usr/bin/captain".to_string(),
        format!("{}/.local/bin/captain", home),
        format!("/root/.local/bin/captain"),
        format!("{}/.cargo/bin/captain", home),
        format!("/home/{}/.shipwreck/bin/captain", user),
        format!("/home/{}/.local/bin/captain", user),
        format!("/home/{}/.cargo/bin/captain", user),
        "captain".to_string(),
        "./captain".to_string(),
    ];
    if let Ok(output) = std::process::Command::new("which").arg("captain").output() {
        if output.status.success() && !output.stdout.is_empty() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                if let Ok(resolved) = std::fs::read_link(&path) {
                    if let Some(parent) = resolved.parent() {
                        if let Some(file_name) = resolved.file_name() {
                            let resolved_path = parent.join(file_name);
                            if resolved_path.exists() {
                                return Some(resolved_path.to_string_lossy().to_string());
                            }
                        }
                    }
                }
                return Some(path);
            }
        }
    }
    for path in &captain_paths {
        if std::path::Path::new(path).is_file() {
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > 0 {
                    if let Ok(resolved) = std::fs::read_link(path) {
                        if let Some(parent) = resolved.parent() {
                            if let Some(file_name) = resolved.file_name() {
                                let resolved_path = parent.join(file_name);
                                if resolved_path.exists() {
                                    return Some(resolved_path.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                    return Some(path.clone());
                }
            }
        }
    }
    None
}
pub fn get_captain_status() -> CaptainStatus {
    if let Ok(mut cache) = CAPTAIN_STATUS.lock() {
        if let Some(status) = cache.as_ref() {
            return status.clone();
        }
    }
    let status = load_captain_status().unwrap_or(CaptainStatus::NotInstalled);
    if let Ok(mut cache) = CAPTAIN_STATUS.lock() {
        *cache = Some(status.clone());
    }
    status
}
pub fn is_captain_available() -> bool {
    matches!(get_captain_status(), CaptainStatus::Installed { .. })
}
pub fn get_captain_path() -> Option<String> {
    match get_captain_status() {
        CaptainStatus::Installed { path, .. } => Some(path),
        _ => find_captain_binary(),
    }
}
pub fn mark_captain_installed(captain_path: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    let status = CaptainStatus::Installed {
        path: captain_path.to_string(),
        installed_at: now.clone(),
        version: None,
        last_verified: now,
    };
    save_captain_status(&status)?;
    if let Ok(mut cache) = CAPTAIN_STATUS.lock() {
        *cache = Some(status);
    }
    Ok(())
}
pub fn mark_captain_corrupted(captain_path: &str, reason: &str) -> Result<()> {
    let status = CaptainStatus::Corrupted {
        path: captain_path.to_string(),
        reason: reason.to_string(),
    };
    save_captain_status(&status)?;
    if let Ok(mut cache) = CAPTAIN_STATUS.lock() {
        *cache = Some(status);
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
        match verify_captain_binary(&captain_path) {
            Ok(_) => {
                mark_captain_installed(&captain_path)?;
                Ok(CaptainStatus::Installed {
                    path: captain_path,
                    installed_at: chrono::Utc::now().to_rfc3339(),
                    version: None,
                    last_verified: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(reason) => {
                mark_captain_corrupted(&captain_path, &reason)?;
                Ok(CaptainStatus::Corrupted {
                    path: captain_path,
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
    match get_captain_status() {
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