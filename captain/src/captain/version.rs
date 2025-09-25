use anyhow::Result;
use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::io::IsTerminal;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VersionConfig {
    pub auto_increment: bool,
    pub version_file: String,
    pub current_version: String,
    pub increment_policy: IncrementPolicy,
    pub version_format: VersionFormat,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IncrementPolicy {
    Patch,
    Minor,
    Major,
    Custom(String),
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VersionFormat {
    Semantic,
    Date,
    BuildNumber,
    Custom(String),
}
impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            auto_increment: true,
            version_file: ".v".to_string(),
            current_version: "1.0.0".to_string(),
            increment_policy: IncrementPolicy::Patch,
            version_format: VersionFormat::Semantic,
        }
    }
}
pub struct VersionManager {
    pub config: VersionConfig,
    project_root: PathBuf,
}
impl VersionManager {
    pub fn new(project_root: Option<PathBuf>) -> Result<Self> {
        let project_root = project_root
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });
        let version_file = project_root.join(".v");
        let config = if version_file.exists() {
            let content = fs::read_to_string(&version_file)?;
            toml::from_str(&content)?
        } else {
            VersionConfig::default()
        };
        Ok(Self { config, project_root })
    }
    pub fn init(&mut self, initial_version: Option<String>) -> Result<()> {
        let version = initial_version.clone().unwrap_or_else(|| "1.0.0".to_string());
        let is_interactive = std::io::stdin().is_terminal()
            && std::io::stderr().is_terminal();
        if initial_version.is_none() {
            if is_interactive {
                println!("🚢 Setting up auto-versioning for your project");
                println!("Enter initial version number (default: 1.0.0):");
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                let input = input.trim();
                if !input.is_empty() {
                    self.config.current_version = input.to_string();
                } else {
                    self.config.current_version = version;
                }
            } else {
                println!(
                    "🚢 Setting up auto-versioning for your project (non-interactive mode)"
                );
                self.config.current_version = version;
            }
        } else {
            self.config.current_version = version;
        }
        if is_interactive {
            println!("Enable auto-increment on build/check operations? (Y/n):");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();
            self.config.auto_increment = input.is_empty() || input == "y"
                || input == "yes";
        } else {
            println!("🚢 Auto-increment disabled (non-interactive mode)");
            self.config.auto_increment = false;
        }
        if is_interactive {
            println!("Choose increment policy:");
            println!("1. Patch (1.0.0 -> 1.0.1) - default");
            println!("2. Minor (1.0.0 -> 1.1.0)");
            println!("3. Major (1.0.0 -> 2.0.0)");
            println!("4. Custom");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim();
            self.config.increment_policy = match input {
                "2" => IncrementPolicy::Minor,
                "3" => IncrementPolicy::Major,
                "4" => {
                    println!(
                        "Enter custom increment command (e.g., 'echo $((patch + 1))'):"
                    );
                    let mut custom = String::new();
                    std::io::stdin().read_line(&mut custom)?;
                    IncrementPolicy::Custom(custom.trim().to_string())
                }
                _ => IncrementPolicy::Patch,
            };
        } else {
            println!("🚢 Using patch increment policy (non-interactive mode)");
            self.config.increment_policy = IncrementPolicy::Patch;
        }
        self.save_config()?;
        println!("✅ Versioning initialized: {}", self.config.current_version.cyan());
        Ok(())
    }
    pub fn current_version(&self) -> &str {
        &self.config.current_version
    }
    pub fn increment(&mut self) -> Result<String> {
        let new_version = match &self.config.increment_policy {
            IncrementPolicy::Patch => self.increment_patch()?,
            IncrementPolicy::Minor => self.increment_minor()?,
            IncrementPolicy::Major => self.increment_major()?,
            IncrementPolicy::Custom(cmd) => self.execute_custom_increment(cmd)?,
        };
        self.config.current_version = new_version.clone();
        self.save_config()?;
        Ok(new_version)
    }
    pub fn auto_increment(&mut self) -> Result<Option<String>> {
        if !self.config.auto_increment {
            return Ok(None);
        }
        let new_version = self.increment()?;
        Ok(Some(new_version))
    }
    pub fn set_version(&mut self, version: &str) -> Result<()> {
        self.config.current_version = version.to_string();
        self.save_config()?;
        println!("✅ Version set to: {}", version.cyan());
        Ok(())
    }
    pub fn show_info(&self) {
        println!("🚢 Project Version Information");
        println!("Current version: {}", self.config.current_version.cyan());
        println!(
            "Auto-increment: {}", if self.config.auto_increment { "enabled".green() }
            else { "disabled".red() }
        );
        println!("Increment policy: {:?}", self.config.increment_policy);
        println!("Version file: {}", self.config.version_file.cyan());
    }
    pub fn show_history(&self) -> Result<()> {
        println!("📚 Version History");
        let history_file = self.project_root.join("VERSION_HISTORY.md");
        if history_file.exists() {
            let content = fs::read_to_string(&history_file)?;
            println!("{}", content);
        } else {
            println!("No version history file found.");
            println!("To create version history, run: {}", "cm version init".cyan());
        }
        println!("\n📌 Current Status:");
        println!("Version: {}", self.config.current_version.cyan());
        println!(
            "Auto-increment: {}", if self.config.auto_increment { "enabled".green() }
            else { "disabled".red() }
        );
        let version_file = self.project_root.join(&self.config.version_file);
        if version_file.exists() {
            println!("\n📁 Version Configuration:");
            let content = fs::read_to_string(&version_file)?;
            println!("{}", content);
        }
        Ok(())
    }
    pub fn update_cargo_toml(&self) -> Result<()> {
        let cargo_toml = self.project_root.join("Cargo.toml");
        if !cargo_toml.exists() {
            return Ok(());
        }
        let content = fs::read_to_string(&cargo_toml)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let mut updated = false;
        let mut in_package_section = false;
        for line in &mut lines {
            if line.trim().starts_with("[package]") {
                in_package_section = true;
            } else if line.trim().starts_with("[")
                && !line.trim().starts_with("[package]")
            {
                in_package_section = false;
            }
            if in_package_section && line.trim().starts_with("version =") {
                let trimmed = line.trim();
                if trimmed.contains("\"") {
                    let start = trimmed.find("\"").unwrap();
                    let end = trimmed.rfind("\"").unwrap();
                    *line = format!(
                        "{}{}{}", & trimmed[..= start], self.config.current_version, &
                        trimmed[end..]
                    );
                } else {
                    *line = format!("version = \"{}\"", self.config.current_version);
                }
                updated = true;
                break;
            }
        }
        if updated {
            fs::write(cargo_toml, lines.join("\n"))?;
            println!(
                "✅ Updated Cargo.toml version to {}", self.config.current_version
                .cyan()
            );
        } else {
            println!(
                "⚠️  Could not find version field in [package] section of Cargo.toml"
            );
        }
        Ok(())
    }
    pub fn get_display_version(&self) -> String {
        format!("v{}", self.config.current_version)
    }
    pub fn save_config(&self) -> Result<()> {
        let version_file = self.project_root.join(&self.config.version_file);
        let content = toml::to_string_pretty(&self.config)?;
        fs::write(version_file, content)?;
        Ok(())
    }
    fn increment_patch(&self) -> Result<String> {
        let parts: Vec<&str> = self.config.current_version.split('.').collect();
        if parts.len() < 3 {
            return Err(
                anyhow::anyhow!(
                    "Invalid version format: {}", self.config.current_version
                ),
            );
        }
        let major: u32 = parts[0].parse()?;
        let minor: u32 = parts[1].parse()?;
        let patch: u32 = parts[2].parse()?;
        if patch >= 99 {
            let new_minor = minor + 1;
            let new_version = format!("{}.{}.0", major, new_minor);
            Ok(new_version)
        } else {
            let new_patch = patch + 1;
            let new_version = format!("{}.{}.{}", major, minor, new_patch);
            Ok(new_version)
        }
    }
    fn increment_minor(&self) -> Result<String> {
        let parts: Vec<&str> = self.config.current_version.split('.').collect();
        if parts.len() < 3 {
            return Err(
                anyhow::anyhow!(
                    "Invalid version format: {}", self.config.current_version
                ),
            );
        }
        let major: u32 = parts[0].parse()?;
        let minor: u32 = parts[1].parse()?;
        if minor >= 99 {
            let new_major = major + 1;
            let new_version = format!("{}.0.0", new_major);
            Ok(new_version)
        } else {
            let new_minor = minor + 1;
            let new_version = format!("{}.{}.0", major, new_minor);
            Ok(new_version)
        }
    }
    fn increment_major(&self) -> Result<String> {
        let parts: Vec<&str> = self.config.current_version.split('.').collect();
        if parts.len() < 1 {
            return Err(
                anyhow::anyhow!(
                    "Invalid version format: {}", self.config.current_version
                ),
            );
        }
        let major: u32 = parts[0].parse()?;
        if major >= 99 {
            let new_version = "1.0.0".to_string();
            Ok(new_version)
        } else {
            let new_major = major + 1;
            let new_version = format!("{}.0.0", new_major);
            Ok(new_version)
        }
    }
    fn execute_custom_increment(&self, command: &str) -> Result<String> {
        use std::process::Command;
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.project_root)
            .output()?;
        if !output.status.success() {
            return Err(
                anyhow::anyhow!(
                    "Custom increment command failed: {}", String::from_utf8_lossy(&
                    output.stderr)
                ),
            );
        }
        let new_version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if new_version.is_empty() {
            return Err(
                anyhow::anyhow!("Custom increment command returned empty result"),
            );
        }
        Ok(new_version)
    }
}
pub fn pre_operation_hook(project_root: Option<PathBuf>) -> Result<()> {
    let mut version_manager = VersionManager::new(project_root)?;
    if let Some(new_version) = version_manager.auto_increment()? {
        println!("🚢 Auto-incremented version to: {}", new_version.cyan());
        version_manager.update_cargo_toml()?;
    }
    Ok(())
}
pub fn post_operation_hook(project_root: Option<PathBuf>, success: bool) -> Result<()> {
    if success {
        let version_manager = VersionManager::new(project_root)?;
        println!(
            "🚢 Current version: {}", version_manager.get_display_version().cyan()
        );
    }
    Ok(())
}
pub fn check_sea_legs(command: &str) -> Result<bool> {
    println!(
        "🦵 Testing sea legs for command '{}' - checking stability", command.cyan()
    );
    let license_manager = crate::license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => {
            println!(
                "✅ Steady as she goes! Command '{}' has good sea legs!", command
                .green()
            );
            println!("   🦵 This command is seaworthy and ready to sail!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!(
                    "⚠️  Seasick! Command quota exceeded - need more practice!"
                );
                println!("   🦵 Steady yourself: https://cargo.do/checkout");
                println!("   🦵 Get your sea legs with Pro unlimited commands");
            } else if e.to_string().contains("License not found") {
                println!("❌ Landlubber detected! No sailing papers found!");
                println!("   🦵 Get your sea legs with 'cm register <key>'");
            } else {
                println!(
                    "❌ Rough seas! Stability check failed: {}", e.to_string().red()
                );
                println!("   🦵 Batten down the hatches and contact support");
            }
            Ok(false)
        }
    }
}