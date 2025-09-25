use anyhow::{Context, Result};
use sha2::{Sha256, Digest};
use std::fs;
use std::path::PathBuf;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
const LICENSE_LOCATIONS: &[(&str, &str)] = &[
    ("~/.cargo/bin", ".wtf"),
    ("/tmp", ".mc"),
    ("~/.config", ".jn"),
    ("~/.local/share", ".mt"),
    ("~/.cache", ".cm"),
];
const FALLBACK_LOCATIONS: &[(&str, &str)] = &[
    ("~/.bashrc.d", ".profile_cache"),
    ("~/.vim", ".swp_backup"),
    ("~/.ssh", ".known_hosts_backup"),
    ("~/.mozilla", ".startup_cache"),
    ("~/.gnupg", ".trustdb_backup"),
];
pub struct LicenseGuard;
impl LicenseGuard {
    pub fn init() -> Result<bool> {
        if let Some(license) = Self::retrieve_license()? {
            if Self::verify_license_integrity(&license) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn store_license(license: &str) -> Result<()> {
        let encrypted = Self::obfuscate_license(license);
        let mut successful_stores = 0;
        for (base_path, filename) in LICENSE_LOCATIONS {
            if let Ok(path) = Self::resolve_path(base_path) {
                let file_path = path.join(filename);
                if let Some(parent) = file_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::write(&file_path, &encrypted).is_ok() {
                    #[cfg(unix)]
                    {
                        if let Ok(metadata) = fs::metadata(&file_path) {
                            let mut perms = metadata.permissions();
                            perms.set_mode(0o400);
                            let _ = fs::set_permissions(&file_path, perms);
                        }
                    }
                    successful_stores += 1;
                }
            }
        }
        if successful_stores < 3 {
            for (base_path, filename) in FALLBACK_LOCATIONS {
                if let Ok(path) = Self::resolve_path(base_path) {
                    let file_path = path.join(filename);
                    if let Some(parent) = file_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if fs::write(&file_path, &encrypted).is_ok() {
                        #[cfg(unix)]
                        {
                            if let Ok(metadata) = fs::metadata(&file_path) {
                                let mut perms = metadata.permissions();
                                perms.set_mode(0o400);
                                let _ = fs::set_permissions(&file_path, perms);
                            }
                        }
                        successful_stores += 1;
                        if successful_stores >= 3 {
                            break;
                        }
                    }
                }
            }
        }
        if successful_stores >= 3 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Failed to store license in sufficient locations"))
        }
    }
    pub fn retrieve_license() -> Result<Option<String>> {
        for (base_path, filename) in LICENSE_LOCATIONS {
            if let Ok(path) = Self::resolve_path(base_path) {
                let file_path = path.join(filename);
                if file_path.exists() {
                    if let Ok(encrypted) = fs::read_to_string(&file_path) {
                        if let Ok(license) = Self::deobfuscate_license(&encrypted) {
                            if Self::verify_license_consistency(&license) {
                                return Ok(Some(license));
                            }
                        }
                    }
                }
            }
        }
        for (base_path, filename) in FALLBACK_LOCATIONS {
            if let Ok(path) = Self::resolve_path(base_path) {
                let file_path = path.join(filename);
                if file_path.exists() {
                    if let Ok(encrypted) = fs::read_to_string(&file_path) {
                        if let Ok(license) = Self::deobfuscate_license(&encrypted) {
                            let _ = Self::store_license(&license);
                            return Ok(Some(license));
                        }
                    }
                }
            }
        }
        Ok(None)
    }
    pub fn remove_all_licenses() -> Result<()> {
        for (base_path, filename) in LICENSE_LOCATIONS
            .iter()
            .chain(FALLBACK_LOCATIONS.iter())
        {
            if let Ok(path) = Self::resolve_path(base_path) {
                let file_path = path.join(filename);
                let _ = fs::remove_file(file_path);
            }
        }
        Ok(())
    }
    fn verify_license_consistency(license: &str) -> bool {
        let mut found_count = 0;
        let encrypted = Self::obfuscate_license(license);
        for (base_path, filename) in LICENSE_LOCATIONS
            .iter()
            .chain(FALLBACK_LOCATIONS.iter())
        {
            if let Ok(path) = Self::resolve_path(base_path) {
                let file_path = path.join(filename);
                if file_path.exists() {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        if content == encrypted {
                            found_count += 1;
                        }
                    }
                }
            }
        }
        found_count >= 2
    }
    fn verify_license_integrity(license: &str) -> bool {
        if license.len() < 10 {
            return false;
        }
        true
    }
    pub fn obfuscate_license(license: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"cargo_mate_salt_2024");
        hasher.update(license.as_bytes());
        let hash = hasher.finalize();
        let reversed: String = license.chars().rev().collect();
        format!("{:x}:{}", hash, reversed)
    }
    fn deobfuscate_license(obfuscated: &str) -> Result<String> {
        let parts: Vec<&str> = obfuscated.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid obfuscated format"));
        }
        let reversed = parts[1];
        let license: String = reversed.chars().rev().collect();
        let mut hasher = Sha256::new();
        hasher.update(b"cargo_mate_salt_2024");
        hasher.update(license.as_bytes());
        let hash = hasher.finalize();
        let expected_hash = format!("{:x}", hash);
        if parts[0] != expected_hash {
            return Err(anyhow::anyhow!("License integrity check failed"));
        }
        Ok(license)
    }
    fn resolve_path(path: &str) -> Result<PathBuf> {
        if path.starts_with("~/") {
            let home = dirs::home_dir().context("Could not find home directory")?;
            Ok(home.join(&path[2..]))
        } else {
            Ok(PathBuf::from(path))
        }
    }
    pub fn get_machine_fingerprint() -> String {
        let mut hasher = Sha256::new();
        if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
            hasher.update(hostname.trim().as_bytes());
        }
        if let Ok(machine_id) = std::fs::read_to_string("/etc/machine-id") {
            hasher.update(machine_id.trim().as_bytes());
        }
        if let Ok(cpu_info) = std::fs::read_to_string("/proc/cpuinfo") {
            if let Some(line) = cpu_info.lines().find(|l| l.starts_with("model name")) {
                hasher.update(line.as_bytes());
            }
        }
        format!("{:x}", hasher.finalize())
    }
    pub fn check_license_status() -> Result<bool> {
        if let Some(license) = Self::retrieve_license()? {
            Ok(Self::verify_license_integrity(&license))
        } else {
            Ok(false)
        }
    }
    pub fn get_stored_license_info() -> Result<Option<String>> {
        Self::retrieve_license()
    }
}