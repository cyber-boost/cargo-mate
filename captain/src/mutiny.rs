use anyhow::{Context, Result};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use crate::captain::license;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MutinyConfig {
    pub overrides: HashMap<String, Override>,
    pub force_flags: Vec<String>,
    pub skip_checks: Vec<String>,
    pub custom_env: HashMap<String, String>,
    pub allow_dirty: bool,
    pub ignore_lockfile: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Override {
    pub enabled: bool,
    pub reason: String,
    pub expires: Option<chrono::DateTime<chrono::Utc>>,
    pub commands: Vec<String>,
}
pub struct MutinyMode {
    config: MutinyConfig,
    config_file: PathBuf,
    active: bool,
}
impl MutinyMode {
    pub fn new() -> Result<Self> {
        let config_file = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("mutiny.toml");
        let config = if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            toml::from_str(&content)?
        } else {
            MutinyConfig::default()
        };
        Ok(Self {
            config,
            config_file,
            active: false,
        })
    }
    pub fn activate(&mut self, reason: &str) -> Result<()> {
        self.active = true;
        println!("🏴‍☠️ {} activated!", "MUTINY MODE".red().bold());
        println!("⚠️  Reason: {}", reason.yellow());
        println!("🔥 Cargo's opinions have been overridden!");
        println!();
        self.log_activation(reason)?;
        Ok(())
    }
    pub fn deactivate(&mut self) -> Result<()> {
        self.active = false;
        println!("✅ Mutiny Mode deactivated");
        println!("🚢 Normal cargo operations restored");
        Ok(())
    }
    pub fn allow_warnings(&mut self) -> Result<()> {
        let override_config = Override {
            enabled: true,
            reason: "Temporarily allowing warnings".to_string(),
            expires: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            commands: vec!["build".to_string(), "test".to_string()],
        };
        self.config.overrides.insert("allow_warnings".to_string(), override_config);
        self.config.force_flags.push("--cap-lints=warn".to_string());
        self.save_config()?;
        println!("⚠️  Warnings will be allowed for the next hour");
        Ok(())
    }
    pub fn skip_tests(&mut self) -> Result<()> {
        self.config.skip_checks.push("test".to_string());
        self.save_config()?;
        println!("🏃 Tests will be skipped");
        Ok(())
    }
    pub fn force_build(&mut self) -> Result<()> {
        self.config.allow_dirty = true;
        self.config.ignore_lockfile = true;
        self.save_config()?;
        println!("💪 Force build enabled - ignoring dirty state and lockfile");
        Ok(())
    }
    pub fn yolo_mode(&mut self) -> Result<()> {
        println!(
            "💀 {} - Disabling ALL safety checks!", "YOLO MODE ACTIVATED".red().bold()
            .blink()
        );
        println!("⚠️  This is extremely dangerous!");
        self.config
            .overrides
            .insert(
                "yolo".to_string(),
                Override {
                    enabled: true,
                    reason: "YOLO - living dangerously".to_string(),
                    expires: Some(chrono::Utc::now() + chrono::Duration::minutes(30)),
                    commands: vec!["*".to_string()],
                },
            );
        self.config.force_flags = vec![
            "--cap-lints=allow".to_string(), "-Z unstable-options".to_string(),
        ];
        self.config.skip_checks = vec![
            "test".to_string(), "clippy".to_string(), "fmt".to_string(), "audit"
            .to_string(),
        ];
        self.config.allow_dirty = true;
        self.config.ignore_lockfile = true;
        self.save_config()?;
        println!("🎲 All bets are off for 30 minutes!");
        Ok(())
    }
    pub fn wrap_cargo_command(&self, args: &[&str]) -> Result<std::process::Output> {
        let mut cmd = Command::new("cargo");
        if self.active {
            for (key, value) in &self.config.custom_env {
                cmd.env(key, value);
            }
            if self.should_skip_command(args.get(0).unwrap_or(&"")) {
                println!("⏭️  Skipping {} due to mutiny override", args[0].yellow());
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    return Ok(std::process::Output {
                        status: std::process::ExitStatus::from_raw(0),
                        stdout: b"Skipped by Mutiny Mode".to_vec(),
                        stderr: Vec::new(),
                    });
                }
                #[cfg(not(unix))]
                {
                    return Ok(std::process::Output {
                        status: std::process::ExitStatus::from(
                            std::process::ExitStatus::default(),
                        ),
                        stdout: b"Skipped by Mutiny Mode".to_vec(),
                        stderr: Vec::new(),
                    });
                }
            }
            cmd.args(args);
            for flag in &self.config.force_flags {
                if !args.contains(&flag.as_str()) {
                    cmd.arg(flag);
                }
            }
            if self.config.allow_dirty && args.get(0) == Some(&"publish") {
                cmd.arg("--allow-dirty");
            }
            if self.config.ignore_lockfile {
                cmd.env("CARGO_IGNORE_LOCKFILE", "1");
            }
        } else {
            cmd.args(args);
        }
        println!("🏴‍☠️ Running: {:?}", cmd);
        cmd.output().context("Failed to execute cargo command")
    }
    fn should_skip_command(&self, command: &str) -> bool {
        self.config.skip_checks.contains(&command.to_string())
    }
    pub fn add_custom_flag(&mut self, flag: &str, reason: &str) -> Result<()> {
        self.config.force_flags.push(flag.to_string());
        let override_config = Override {
            enabled: true,
            reason: reason.to_string(),
            expires: None,
            commands: vec!["*".to_string()],
        };
        self.config
            .overrides
            .insert(format!("custom_flag_{}", flag.replace("-", "_")), override_config);
        self.save_config()?;
        println!("➕ Added custom flag: {}", flag.green());
        Ok(())
    }
    pub fn set_env(&mut self, key: &str, value: &str) -> Result<()> {
        self.config.custom_env.insert(key.to_string(), value.to_string());
        self.save_config()?;
        println!("🔧 Set environment variable: {}={}", key.cyan(), value);
        Ok(())
    }
    pub fn status(&self) {
        println!("{}", "=== Mutiny Mode Status ===".red().bold());
        if self.active {
            println!("Status: {} ACTIVE", "🏴‍☠️".red());
        } else {
            println!("Status: {} Inactive", "🚢".green());
        }
        if !self.config.overrides.is_empty() {
            println!("\n📋 Active Overrides:");
            for (name, override_config) in &self.config.overrides {
                if override_config.enabled {
                    println!("   {} - {}", name.yellow(), override_config.reason);
                    if let Some(expires) = override_config.expires {
                        let remaining = expires - chrono::Utc::now();
                        println!(
                            "      Expires in: {} minutes", remaining.num_minutes()
                        );
                    }
                }
            }
        }
        if !self.config.force_flags.is_empty() {
            println!("\n🚩 Forced Flags:");
            for flag in &self.config.force_flags {
                println!("   {}", flag.cyan());
            }
        }
        if !self.config.skip_checks.is_empty() {
            println!("\n⏭️  Skipped Checks:");
            for check in &self.config.skip_checks {
                println!("   {}", check.yellow());
            }
        }
        if !self.config.custom_env.is_empty() {
            println!("\n🔧 Custom Environment:");
            for (key, value) in &self.config.custom_env {
                println!("   {}={}", key.cyan(), value);
            }
        }
        if self.config.allow_dirty {
            println!("\n⚠️  Allowing dirty repository");
        }
        if self.config.ignore_lockfile {
            println!("⚠️  Ignoring Cargo.lock");
        }
    }
    pub fn clean_expired(&mut self) -> Result<()> {
        let now = chrono::Utc::now();
        let mut expired = Vec::new();
        for (name, override_config) in &self.config.overrides {
            if let Some(expires) = override_config.expires {
                if expires < now {
                    expired.push(name.clone());
                }
            }
        }
        for name in expired {
            self.config.overrides.remove(&name);
            println!("🧹 Cleaned expired override: {}", name);
        }
        self.save_config()?;
        Ok(())
    }
    pub fn reset(&mut self) -> Result<()> {
        self.config = MutinyConfig::default();
        self.active = false;
        self.save_config()?;
        println!("🔄 Mutiny Mode configuration reset to defaults");
        Ok(())
    }
    fn save_config(&self) -> Result<()> {
        let toml = toml::to_string_pretty(&self.config)?;
        fs::write(&self.config_file, toml)?;
        Ok(())
    }
    fn log_activation(&self, reason: &str) -> Result<()> {
        let log_file = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("mutiny.log");
        let entry = format!(
            "[{}] Activated: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            reason
        );
        let mut file = fs::OpenOptions::new().create(true).append(true).open(log_file)?;
        use std::io::Write;
        file.write_all(entry.as_bytes())?;
        Ok(())
    }
}
impl Default for MutinyConfig {
    fn default() -> Self {
        Self {
            overrides: HashMap::new(),
            force_flags: Vec::new(),
            skip_checks: Vec::new(),
            custom_env: HashMap::new(),
            allow_dirty: false,
            ignore_lockfile: false,
        }
    }
}
pub struct MutinyGuard {
    mode: MutinyMode,
}
impl MutinyGuard {
    pub fn new(reason: &str) -> Result<Self> {
        let mut mode = MutinyMode::new()?;
        mode.activate(reason)?;
        Ok(Self { mode })
    }
}
impl Drop for MutinyGuard {
    fn drop(&mut self) {
        let _ = self.mode.deactivate();
    }
}
pub fn check_helmsman_direction(command: &str) -> Result<bool> {
    println!(
        "🧭 Helmsman checking course for command '{}' - steady as she goes!", command
        .cyan()
    );
    let license_manager = license::LicenseManager::new();
    match license_manager?.enforce_license(command) {
        Ok(_) => {
            println!(
                "✅ Helmsman reports: Command '{}' on correct heading!", command.green()
            );
            println!("   🧭 All systems aligned - ready to steer!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!("⚠️  Helmsman warning: Course deviation detected!");
                println!("   🧭 Correct heading: https://cargo.do/checkout");
                println!("   🧭 Adjust course for unlimited navigation");
            } else if e.to_string().contains("License not found") {
                println!("❌ Helmsman emergency: No navigation coordinates!");
                println!("   🧭 Plot course with 'cm register <key>'");
            } else {
                println!(
                    "❌ Helmsman distress: Course check failed: {}", e.to_string().red()
                );
                println!("   🧭 Man your stations - prepare to heave to!");
            }
            Ok(false)
        }
    }
}