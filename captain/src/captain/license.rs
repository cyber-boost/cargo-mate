use anyhow::{Context, Result};
use reqwest::blocking;
use serde::{Deserialize, Serialize};
use std::env;
use chrono::{DateTime, Utc};
use std::fs;
use std::path::PathBuf;
use crate::license_guard::LicenseGuard;
use crate::log::Log;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[derive(Debug, Deserialize)]
pub struct LicenseValidation {
    pub valid: bool,
    pub tier: String,
    pub remaining: Option<i32>,
    pub used: Option<i32>,
    pub unlimited: Option<bool>,
    pub expires_at: Option<String>,
    pub error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct LicenseRequest {
    pub license_key: String,
    pub command: String,
    pub timestamp: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct LicenseCheckRequest {
    pub license_key: String,
    pub action: String,
}
pub struct LicenseManager {
    api_base_url: String,
}
impl LicenseManager {
    pub fn new() -> Result<Self> {
        dotenvy::dotenv().ok();
        let api_base_url = env::var("CARGO_MATE_API")
            .unwrap_or_else(|_| "https://cargo.do/api".to_string());
        Ok(Self { api_base_url })
    }
    pub fn register_license(&self, license_key: &str) -> Result<()> {
        let log = Log::new();
        log.log(
            "Registering license",
            vec!["license".to_string(), "register".to_string()],
        )?;
        log.log(
            "Note: Licenses are now automatically linked to users when purchased.",
            vec!["license".to_string(), "register".to_string()],
        )?;
        log.log(
            "This command is for linking existing licenses to your current installation.",
            vec!["license".to_string(), "register".to_string()],
        )?;
        if license_key.len() < 10 {
            log.log(
                "Invalid license format",
                vec!["license".to_string(), "register".to_string()],
            )?;
            return Err(
                anyhow::anyhow!("Invalid license format. Expected: <license_string>"),
            );
        }
        let user_id = self.get_or_create_user_id()?;
        log.log("User ID", vec!["license".to_string(), "register".to_string()])?;
        let affiliate_code: Option<String> = None;
        let client = blocking::Client::new();
        let mut payload = serde_json::json!(
            { "license_key" : license_key, "user_id" : user_id }
        );
        if let Some(afl_code) = affiliate_code {
            payload["affiliate_code"] = serde_json::json!(afl_code);
        }
        let url = format!("{}/register-license", self.api_base_url);
        log.log(
            "Calling API endpoint",
            vec!["license".to_string(), "register".to_string()],
        )?;
        let response = client
            .post(&url)
            .json(&payload)
            .send()
            .context("Failed to connect to license registration API")?;
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .unwrap_or_else(|_| "Unknown error".to_string());
            log.log(
                "Registration failed",
                vec!["license".to_string(), "register".to_string()],
            )?;
            crate::wtf::display_api_failure_art();
            std::process::exit(1);
        }
        let registration_result: serde_json::Value = response
            .json()
            .context("Failed to parse license registration response")?;
        if registration_result["success"] != true {
            let error_msg = registration_result["error"]
                .as_str()
                .unwrap_or("Unknown registration error");
            return Err(anyhow::anyhow!("License registration failed: {}", error_msg));
        }
        let tier = registration_result["tier"].as_str().unwrap_or("FREE").to_string();
        self.save_local_license(license_key, &tier)?;
        log.log(
            "License registered successfully",
            vec!["license".to_string(), "register".to_string()],
        )?;
        log.log("Tier", vec!["license".to_string(), "register".to_string()])?;
        log.log("User", vec!["license".to_string(), "register".to_string()])?;
        let obfuscated_license = LicenseGuard::obfuscate_license(license_key);
        log.log(
            "Obfuscated license",
            vec!["license".to_string(), "register".to_string()],
        )?;
        if let Err(e) = LicenseGuard::store_license(&obfuscated_license) {
            log.log(
                "Error storing license",
                vec!["license".to_string(), "register".to_string()],
            )?;
        }
        log.log("License stored", vec!["license".to_string(), "register".to_string()])?;
        Ok(())
    }
    pub fn show_user_info(&self) -> Result<()> {
        let log = Log::new();
        let user_id = self.get_or_create_user_id()?;
        log.log("User Information", vec!["license".to_string(), "show".to_string()])?;
        log.log("User ID", vec!["license".to_string(), "show".to_string()])?;
        let license_key = match LicenseGuard::retrieve_license() {
            Ok(Some(license)) => license,
            Ok(None) => {
                log.log(
                    "License not found",
                    vec!["license".to_string(), "show".to_string()],
                )?;
                return Ok(());
            }
            Err(_) => {
                log.log(
                    "Failed to retrieve license",
                    vec!["license".to_string(), "show".to_string()],
                )?;
                return Ok(());
            }
        };
        log.log("License", vec!["license".to_string(), "show".to_string()])?;
        match self.check_license_status() {
            Ok(validation) => {
                log.log("Tier", vec!["license".to_string(), "show".to_string()])?;
                if let Some(expires_at) = validation.expires_at {
                    log.log("Expires", vec!["license".to_string(), "show".to_string()])?;
                }
                if validation.tier == "FREE" {
                    if let Some(remaining) = validation.remaining {
                        log.log(
                            "Remaining commands",
                            vec!["license".to_string(), "show".to_string()],
                        )?;
                    }
                }
            }
            Err(e) => {
                log.log(
                    "License status check failed",
                    vec!["license".to_string(), "show".to_string()],
                )?;
            }
        }
        Ok(())
    }
    pub fn check_license_status(&self) -> Result<LicenseValidation> {
        let license_key = self.get_local_license()?;
        let user_id = self.get_or_create_user_id()?;
        let client = blocking::Client::new();
        let response = client
            .post(&format!("{}/licenses/validate", self.api_base_url))
            .json(
                &serde_json::json!(
                    { "license_key" : license_key, "user_id" : user_id, "action" :
                    "check_status" }
                ),
            )
            .send()
            .context("Failed to connect to license API")?;
        if response.status().is_success() {
            let validation: LicenseValidation = response
                .json()
                .context("Failed to parse license validation response")?;
            Ok(validation)
        } else {
            Ok(LicenseValidation {
                valid: false,
                tier: "FREE".to_string(),
                remaining: Some(0),
                used: None,
                unlimited: Some(false),
                expires_at: None,
                error: Some("License not found or inactive".to_string()),
            })
        }
    }
    pub fn record_usage(&self, command: &str) -> Result<()> {
        let log = Log::new();
        let license_key = self.get_local_license()?;
        let user_id = self.get_or_create_user_id()?;
        let client = blocking::Client::new();
        let response = client
            .post(&format!("{}/licenses/validate", self.api_base_url))
            .json(
                &serde_json::json!(
                    { "license_key" : license_key, "user_id" : user_id, "action" :
                    "use_command" }
                ),
            )
            .timeout(std::time::Duration::from_secs(3))
            .send();
        match response {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => {
                let status_code = resp.status();
                log.log(
                    "Warning: Failed to record command usage (HTTP {})",
                    vec!["license".to_string(), "record_usage".to_string()],
                )?;
                log.log(
                    "Your command was executed but usage may not be properly tracked.",
                    vec!["license".to_string(), "record_usage".to_string()],
                )?;
                Ok(())
            }
            Err(e) => {
                log.log(
                    "Warning: Could not connect to license server to record usage",
                    vec!["license".to_string(), "record_usage".to_string()],
                )?;
                log.log(
                    "Your command was executed but usage may not be properly tracked.",
                    vec!["license".to_string(), "record_usage".to_string()],
                )?;
                eprintln!(
                    "⚠️  Your command was executed but usage may not be properly tracked."
                );
                Ok(())
            }
        }
    }
    pub fn enforce_license(&self, command: &str) -> Result<()> {
        let log = Log::new();
        let user_id = self.get_or_create_user_id()?;
        let license_key = match LicenseGuard::retrieve_license() {
            Ok(key) => key,
            Err(_) => {
                if let Ok(Some(key)) = LicenseGuard::retrieve_license() {
                    let home_dir = dirs::home_dir().unwrap();
                    let config_dir = home_dir.join(".shipwreck");
                    let license_file = config_dir.join("license.key");
                    let _ = fs::write(&license_file, &key);
                    Some(key)
                } else {
                    eprintln!(
                        "┌─────────────────────────────────────────────────────┐"
                    );
                    eprintln!(
                        "│  ❌ FATAL: No valid license found                  │"
                    );
                    eprintln!(
                        "├─────────────────────────────────────────────────────┤"
                    );
                    eprintln!(
                        "│  Cargo Mate requires a license to operate.         │"
                    );
                    eprintln!(
                        "│  Auto-registration may have failed.                │"
                    );
                    eprintln!(
                        "│                                                     │"
                    );
                    eprintln!(
                        "│  Please reinstall to get your FREE license:        │"
                    );
                    eprintln!(
                        "│  curl -sSL https://get.cargo.do/mate | bash       │"
                    );
                    eprintln!(
                        "└─────────────────────────────────────────────────────┘"
                    );
                    std::process::exit(1);
                }
            }
        };
        let validation = match self.check_license_status() {
            Ok(validation) => validation,
            Err(_) => {
                match LicenseGuard::get_stored_license_info() {
                    Ok(Some(_)) => {
                        LicenseValidation {
                            valid: true,
                            tier: "PRO".to_string(),
                            remaining: None,
                            used: None,
                            unlimited: Some(true),
                            expires_at: None,
                            error: None,
                        }
                    }
                    _ => {
                        log.log(
                            "No internet connection. FREE tier requires online verification.",
                            vec!["license".to_string(), "enforce_license".to_string()],
                        )?;
                        std::process::exit(1);
                    }
                }
            }
        };
        if !validation.valid {
            if let Some(error) = validation.error {
                log.log(
                    "License validation failed",
                    vec!["license".to_string(), "enforce_license".to_string()],
                )?;
            }
            log.log(
                "Your license key",
                vec!["license".to_string(), "enforce_license".to_string()],
            )?;
            std::process::exit(1);
        }
        if validation.tier == "FREE" {
            let api_remaining = validation.remaining.unwrap_or(0);
            let local_count = self.get_local_command_count()?;
            let effective_remaining = if api_remaining <= local_count {
                api_remaining
            } else {
                10 - local_count
            };
            if effective_remaining <= 0 {
                eprintln!(
                    "┌─────────────────────────────────────────────────────┐"
                );
                eprintln!("│  :( DAILY LIMIT EXCEEDED 0/10 commands ):           │");
                eprintln!(
                    "├─────────────────────────────────────────────────────┤"
                );
                eprintln!("│  FREE tier limit exhausted for today.               │");
                eprintln!("│                                                     │");
                eprintln!("│  Options:                                           │");
                eprintln!(
                    "│  • Wait until midnight UTC for reset                │"
                );
                eprintln!(
                    "│  • Upgrade to PRO: https://cargo.do/pro             │"
                );
                eprintln!(
                    "└─────────────────────────────────────────────────────┘"
                );
                std::process::exit(1);
            }
        }
        self.record_usage(command)?;
        if validation.tier == "FREE" {
            match self.increment_local_command_count() {
                Ok(_) => {}
                Err(e) => {}
            }
        }
        Ok(())
    }
    pub fn get_or_create_user_id(&self) -> Result<String> {
        let log = Log::new();
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home_dir.join(".shipwreck");
        let user_id_file = config_dir.join("user.id");
        if user_id_file.exists() {
            let user_id = fs::read_to_string(&user_id_file)
                .context("Failed to read user ID file")?
                .trim()
                .to_string();
            Ok(user_id)
        } else {
            fs::create_dir_all(&config_dir)
                .context("Failed to create config directory")?;
            let user_id = format!(
                "cm_{}_{}", Utc::now().timestamp(), std::process::id()
            );
            let client = blocking::Client::new();
            let payload = serde_json::json!({ "user_id" : user_id.clone() });
            match client
                .post(&format!("{}/users/register", self.api_base_url))
                .json(&payload)
                .timeout(std::time::Duration::from_secs(5))
                .send()
            {
                Ok(response) if response.status().is_success() => {
                    if let Ok(json) = response.json::<serde_json::Value>() {
                        fs::write(&user_id_file, &user_id)
                            .context("Failed to save user ID file")?;
                        if let Some(license_key) = json["license_key"].as_str() {
                            let license_file = config_dir.join("license.key");
                            fs::write(&license_file, license_key)
                                .context("Failed to save license file")?;
                            if let Some(tier) = json["license_tier"].as_str() {
                                let tier_file = config_dir.join("license.tier");
                                fs::write(&tier_file, tier)
                                    .context("Failed to save tier file")?;
                            }
                            if let Err(e) = crate::license_guard::LicenseGuard::store_license(
                                license_key,
                            ) {
                                log.log(
                                    "Warning: Could not store license in hidden locations",
                                    vec!["license".to_string(), "register".to_string()],
                                )?;
                            }
                            log.log(
                                "User registered successfully",
                                vec!["license".to_string(), "register".to_string()],
                            )?;
                            log.log(
                                "User ID",
                                vec!["license".to_string(), "register".to_string()],
                            )?;
                            log.log(
                                "License",
                                vec!["license".to_string(), "register".to_string()],
                            )?;
                            log.log(
                                "Affiliate program info",
                                vec!["license".to_string(), "register".to_string()],
                            )?;
                            let result: Result<(), anyhow::Error> = Ok(());
                            if let Err(e) = result {
                                log.log(
                                    "Warning: Could not show affiliate info",
                                    vec!["license".to_string(), "register".to_string()],
                                )?;
                            }
                            if let Some(afl_code) = json["affiliate_code"].as_str() {
                                let afl_file = config_dir.join("affiliate_code");
                                let _ = fs::write(&afl_file, afl_code);
                                log.log(
                                    "Affiliate Code",
                                    vec!["license".to_string(), "register".to_string()],
                                )?;
                            }
                        }
                    }
                }
                _ => {
                    fs::write(&user_id_file, &user_id)
                        .context("Failed to save user ID file")?;
                    log.log(
                        "Could not connect to license server. Running in offline mode",
                        vec!["license".to_string(), "register".to_string()],
                    )?;
                }
            }
            Ok(user_id)
        }
    }
    fn get_ip_address(&self) -> String {
        match self.fetch_external_ip() {
            Ok(ip) => ip,
            Err(_) => {
                std::env::var("SSH_CLIENT")
                    .or_else(|_| std::env::var("REMOTE_ADDR"))
                    .unwrap_or_else(|_| "127.0.0.1".to_string())
                    .split_whitespace()
                    .next()
                    .unwrap_or("127.0.0.1")
                    .to_string()
            }
        }
    }
    fn fetch_external_ip(&self) -> Result<String> {
        let client = blocking::Client::new();
        let response = client
            .get("https://api.ipify.org")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .context("Failed to fetch external IP")?;
        if response.status().is_success() {
            let ip = response
                .text()
                .context("Failed to read IP response")?
                .trim()
                .to_string();
            Ok(ip)
        } else {
            Err(anyhow::anyhow!("Failed to get external IP"))
        }
    }
    pub fn get_local_license(&self) -> Result<String> {
        let log = Log::new();
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home_dir.join(".shipwreck");
        let license_file = config_dir.join("license.key");
        if !license_file.exists() {
            log.log(
                "No license found. Run 'cm register <license-key>' to register your Pro license",
                vec!["license".to_string(), "get_local_license".to_string()],
            )?;
            return Err(
                anyhow::anyhow!(
                    "No license found. Run 'cm register <license-key>' to register your Pro license"
                ),
            );
        }
        let license = fs::read_to_string(license_file)
            .context("Failed to read license file")?
            .trim()
            .to_string();
        Ok(license)
    }
    fn save_local_license(&self, license_key: &str, tier: &str) -> Result<()> {
        let log = Log::new();
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home_dir.join(".shipwreck");
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        let license_file = config_dir.join("license.key");
        fs::write(&license_file, license_key).context("Failed to save license file")?;
        let tier_file = config_dir.join("license.tier");
        fs::write(&tier_file, tier).context("Failed to save license tier")?;
        let obfuscated_license = LicenseGuard::obfuscate_license(license_key);
        if let Err(e) = LicenseGuard::store_license(&obfuscated_license) {
            log.log(
                "Warning: Could not store license in hidden locations",
                vec!["license".to_string(), "save_local_license".to_string()],
            )?;
        }
        log.log(
            "License saved",
            vec!["license".to_string(), "save_local_license".to_string()],
        )?;
        Ok(())
    }
    pub fn get_stored_license_info(&self) -> Result<(String, String)> {
        let log = Log::new();
        let license_key = self.get_local_license()?;
        let home_dir = dirs::home_dir().unwrap();
        let config_dir = home_dir.join(".shipwreck");
        let tier_file = config_dir.join("license.tier");
        let tier = if tier_file.exists() {
            fs::read_to_string(tier_file)?.trim().to_string()
        } else {
            "FREE".to_string()
        };
        Ok((license_key, tier))
    }
    pub fn get_license_info(&self) -> Result<serde_json::Value> {
        let license_key = self.get_local_license()?;
        let client = blocking::Client::new();
        let response = client
            .post(&format!("{}/licenses/info", self.api_base_url))
            .json(&serde_json::json!({ "license_key" : license_key }))
            .send()
            .context("Failed to connect to license API")?;
        if response.status().is_success() {
            let info: serde_json::Value = response
                .json()
                .context("Failed to parse license info response")?;
            Ok(info)
        } else {
            Err(anyhow::anyhow!("Failed to get license info: {}", response.status()))
        }
    }
    pub fn check_remaining_commands(&self) -> Result<i32> {
        let locations = Self::get_counter_locations();
        let mut incremented = false;
        for (base_path, filename) in &locations {
            if let Ok(path) = self.resolve_counter_path(base_path, filename) {
                let current_count = if path.exists() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(count) = self.decrypt_counter(&content) {
                            count
                        } else {
                            0
                        }
                    } else {
                        0
                    }
                } else {
                    0
                };
                let new_count = current_count + 1;
                let encrypted = self.encrypt_counter(new_count)?;
                if fs::write(&path, &encrypted).is_ok() {
                    incremented = true;
                }
            }
        }
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let config_dir = home_dir.join(".shipwreck");
        let old_counter_file = config_dir.join("daily_commands");
        let old_count = if old_counter_file.exists() {
            if let Ok(content) = fs::read_to_string(&old_counter_file) {
                content.trim().parse::<i32>().unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };
        let _ = fs::write(&old_counter_file, format!("{}", old_count + 1));
        let validation = self.check_license_status()?;
        if !validation.valid {
            return Err(
                anyhow::anyhow!(
                    validation.error.unwrap_or_else(|| "License validation failed"
                    .to_string())
                ),
            );
        }
        match validation.tier.as_str() {
            "PRO" => Ok(-1),
            "FREE" => Ok(validation.remaining.unwrap_or(0)),
            _ => Ok(0),
        }
    }
    pub fn is_license_expired(&self) -> Result<bool> {
        let license_key = self.get_local_license()?;
        let client = blocking::Client::new();
        let response = client
            .post(&format!("{}/licenses/check_expiration", self.api_base_url))
            .json(&serde_json::json!({ "license_key" : license_key }))
            .send()
            .context("Failed to connect to license API")?;
        if response.status().is_success() {
            let result: serde_json::Value = response
                .json()
                .context("Failed to parse expiration check response")?;
            Ok(result["expired"].as_bool().unwrap_or(true))
        } else {
            Ok(true)
        }
    }
    fn get_counter_locations() -> Vec<(String, String)> {
        let mut locations = Vec::new();
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let os_type = std::env::consts::OS;
        let is_windows = os_type == "windows";
        let is_macos = os_type == "macos";
        if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
            locations.push((format!("{}/bin", cargo_home), ".cmd_count".to_string()));
        } else {
            if is_windows {
                locations
                    .push((
                        "%USERPROFILE%\\.cargo\\bin".to_string(),
                        ".cmd_count".to_string(),
                    ));
            } else {
                locations.push(("~/.cargo/bin".to_string(), ".cmd_count".to_string()));
            }
        }
        if is_windows {
            locations.push(("%APPDATA%".to_string(), ".cargo_count".to_string()));
        } else if is_macos {
            locations
                .push(("~/Library/Preferences".to_string(), ".cargo_count".to_string()));
            locations
                .push((
                    "~/Library/Application Support/cargo-mate".to_string(),
                    ".cargo_count".to_string(),
                ));
            if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
                locations
                    .push((
                        format!("{}/cargo-mate", xdg_config),
                        ".cargo_count".to_string(),
                    ));
            }
        } else {
            if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
                locations
                    .push((
                        format!("{}/cargo-mate", xdg_config),
                        ".cargo_count".to_string(),
                    ));
            } else {
                locations
                    .push((
                        "~/.config/cargo-mate".to_string(),
                        ".cargo_count".to_string(),
                    ));
                locations
                    .push(("~/.cargo-mate".to_string(), ".cargo_count".to_string()));
            }
        }
        if is_windows {
            locations.push(("%LOCALAPPDATA%".to_string(), ".build_count".to_string()));
        } else if is_macos {
            locations
                .push((
                    "~/Library/Application Support/cargo-mate".to_string(),
                    ".build_count".to_string(),
                ));
            locations
                .push((
                    "~/Library/Preferences/cargo-mate".to_string(),
                    ".build_count".to_string(),
                ));
            if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
                locations
                    .push((
                        format!("{}/cargo-mate", xdg_data),
                        ".build_count".to_string(),
                    ));
            }
        } else {
            if let Ok(xdg_data) = std::env::var("XDG_DATA_HOME") {
                locations
                    .push((
                        format!("{}/cargo-mate", xdg_data),
                        ".build_count".to_string(),
                    ));
            } else {
                locations
                    .push((
                        "~/.local/share/cargo-mate".to_string(),
                        ".build_count".to_string(),
                    ));
                locations
                    .push(("~/.cargo-mate".to_string(), ".build_count".to_string()));
                locations
                    .push((
                        "~/.local/cargo-mate".to_string(),
                        ".build_count".to_string(),
                    ));
            }
        }
        if is_windows {
            locations.push(("%TEMP%".to_string(), ".cm_usage".to_string()));
        } else if is_macos {
            locations
                .push((
                    "~/Library/Caches/cargo-mate".to_string(),
                    ".cm_usage".to_string(),
                ));
            locations
                .push((
                    "~/Library/Application Support/cargo-mate/cache".to_string(),
                    ".cm_usage".to_string(),
                ));
            if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
                locations
                    .push((
                        format!("{}/cargo-mate", xdg_cache),
                        ".cm_usage".to_string(),
                    ));
            }
        } else {
            if let Ok(xdg_cache) = std::env::var("XDG_CACHE_HOME") {
                locations
                    .push((
                        format!("{}/cargo-mate", xdg_cache),
                        ".cm_usage".to_string(),
                    ));
            } else {
                locations
                    .push(("~/.cache/cargo-mate".to_string(), ".cm_usage".to_string()));
                locations
                    .push(("~/.cargo-mate/cache".to_string(), ".cm_usage".to_string()));
                locations.push(("/tmp/cargo-mate".to_string(), ".cm_usage".to_string()));
            }
        }
        if let Ok(temp_dir) = std::env::var("TMPDIR") {
            locations.push((temp_dir, ".daily_limit".to_string()));
        } else if let Ok(temp_dir) = std::env::var("TEMP") {
            locations.push((temp_dir, ".daily_limit".to_string()));
        } else {
            locations.push(("/tmp".to_string(), ".daily_limit".to_string()));
        }
        locations
    }
    fn get_local_command_count(&self) -> Result<i32> {
        let mut max_count = 0;
        let mut found_any = false;
        let locations = Self::get_counter_locations();
        for (base_path, filename) in &locations {
            if let Ok(path) = self.resolve_counter_path(&base_path, &filename) {
                if path.exists() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(decrypted) = self.decrypt_counter(&content) {
                            max_count = max_count.max(decrypted);
                            found_any = true;
                        }
                    }
                }
            }
        }
        if found_any { Ok(max_count) } else { Ok(0) }
    }
    fn increment_local_command_count(&self) -> Result<()> {
        let current_count = self.get_local_command_count()?;
        let new_count = current_count + 1;
        let encrypted_count = self.encrypt_counter(new_count)?;
        let locations = Self::get_counter_locations();
        let mut successful_stores = 0;
        for (base_path, filename) in &locations {
            if let Ok(path) = self.resolve_counter_path(&base_path, &filename) {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::write(&path, &encrypted_count).is_ok() {
                    #[cfg(unix)]
                    {
                        if let Ok(metadata) = fs::metadata(&path) {
                            let mut perms = metadata.permissions();
                            perms.set_mode(0o400);
                            let _ = fs::set_permissions(&path, perms);
                        }
                    }
                    successful_stores += 1;
                }
            }
        }
        if successful_stores == 0 {
            return Err(
                anyhow::anyhow!("Failed to store command counter in any location"),
            );
        }
        Ok(())
    }
    fn resolve_counter_path(&self, base_path: &str, filename: &str) -> Result<PathBuf> {
        let expanded = if base_path.starts_with("~/") {
            let home = dirs::home_dir().context("Could not find home directory")?;
            home.join(&base_path[2..]).join(filename)
        } else if base_path.starts_with("%") && base_path.ends_with("%") {
            let env_var = &base_path[1..base_path.len() - 1];
            if let Ok(env_path) = std::env::var(env_var) {
                PathBuf::from(env_path).join(filename)
            } else {
                PathBuf::from(base_path).join(filename)
            }
        } else {
            PathBuf::from(base_path).join(filename)
        };
        Ok(expanded)
    }
    fn encrypt_counter(&self, count: i32) -> Result<String> {
        let salt = 0x1337;
        let obfuscated = ((count as u32) ^ salt).to_string();
        Ok(obfuscated.chars().rev().collect())
    }
    fn decrypt_counter(&self, encrypted: &str) -> Result<i32> {
        let reversed: String = encrypted.chars().rev().collect();
        let obfuscated: u32 = reversed.parse()?;
        let salt = 0x1337;
        let count = (obfuscated ^ salt) as i32;
        Ok(count)
    }
    pub fn reset_local_command_count(&self) -> Result<()> {
        let encrypted_zero = self.encrypt_counter(0)?;
        let locations = Self::get_counter_locations();
        for (base_path, filename) in &locations {
            if let Ok(path) = self.resolve_counter_path(base_path, filename) {
                if path.exists() {
                    let _ = fs::write(&path, &encrypted_zero);
                }
            }
        }
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home_dir.join(".shipwreck");
        let old_counter_file = config_dir.join("daily_commands");
        if old_counter_file.exists() {
            let _ = fs::write(&old_counter_file, "0");
        }
        Ok(())
    }
    pub fn debug_command_counters(&self) -> Result<()> {
        let log = Log::new();
        let local_count = self.get_local_command_count()?;
        let locations = Self::get_counter_locations();
        log.log(
            "🔍 Command Counter Debug:",
            vec!["license".to_string(), "debug_command_counters".to_string()],
        )?;
        log.log(
            "   📁 Local counter",
            vec!["license".to_string(), "debug_command_counters".to_string()],
        )?;
        log.log(
            "   🖥️  Platform",
            vec!["license".to_string(), "debug_command_counters".to_string()],
        )?;
        log.log(
            "   🔐 Dynamic locations",
            vec!["license".to_string(), "debug_command_counters".to_string()],
        )?;
        for (base_path, filename) in &locations {
            if let Ok(path) = self.resolve_counter_path(base_path, filename) {
                let status = if path.exists() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(count) = self.decrypt_counter(&content) {
                            format!("✅ {} commands", count)
                        } else {
                            "❌ Corrupted".to_string()
                        }
                    } else {
                        "❌ Unreadable".to_string()
                    }
                } else {
                    "❌ Missing".to_string()
                };
                let display_path = if base_path.contains("~") {
                    format!("~{}", & base_path[1..])
                } else {
                    base_path.to_string()
                };
                println!("      {} -> {}: {}", filename, display_path, status);
            } else {
                println!("      {} -> {}: ❌ Invalid path", filename, base_path);
            }
        }
        match self.check_license_status() {
            Ok(validation) => {
                if validation.tier == "FREE" {
                    let api_remaining = validation.remaining.unwrap_or(0);
                    let api_used = validation.used.unwrap_or(0);
                    println!("   🗄️  API remaining: {} commands", api_remaining);
                    println!("   📊 API used: {} commands", api_used);
                    println!(
                        "   📈 Total API limit: {} commands", api_used + api_remaining
                    );
                } else {
                    println!("   🌟 PRO tier: Unlimited commands");
                }
            }
            Err(e) => {
                println!("   ❌ API counter: Unavailable ({})", e);
            }
        }
        Ok(())
    }
    pub fn get_user_tier(&self) -> Result<String> {
        let user_id = self.get_or_create_user_id()?;
        let home_dir = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home_dir.join(".shipwreck");
        let tier_file = config_dir.join("license.tier");
        if tier_file.exists() {
            let tier = fs::read_to_string(&tier_file)?.trim().to_string();
            if !tier.is_empty() {
                return Ok(tier);
            }
        }
        Ok("FREE".to_string())
    }
    pub fn get_remaining_commands(&self) -> Result<i32> {
        let validation = self.check_license_status()?;
        if validation.tier == "FREE" {
            let api_remaining = validation.remaining.unwrap_or(0);
            let local_count = self.get_local_command_count()?;
            let effective_remaining = if api_remaining <= local_count {
                api_remaining
            } else {
                10 - local_count
            };
            Ok(effective_remaining.max(0))
        } else {
            Ok(i32::MAX)
        }
    }
}