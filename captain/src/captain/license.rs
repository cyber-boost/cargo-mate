use anyhow::{Context, Result};
use std::process::Command;
use colored::*;
pub struct LicenseManager {}
impl LicenseManager {
    pub fn new() -> Self {
        Self {}
    }
    pub fn check_license(&self) -> Result<bool> {
        println!(
            "🔐 {}", "License verification requires the advanced captain binary"
            .bright_blue()
        );
        println!("   Delegating to captain for license validation...");
        self.delegate_to_captain(vec!["license", "check"])
    }
    pub fn enforce_license(&self, command: &str) -> Result<bool> {
        println!(
            "🔐 {}", format!("License enforcement for '{}' requires captain binary",
            command) .bright_blue()
        );
        println!("   Delegating to captain for license enforcement...");
        self.delegate_to_captain(vec!["license", "enforce", command])
    }
    pub fn register_license(&self, license_key: &str) -> Result<()> {
        println!("🔐 {}", format!("Registering license: {}", license_key) .yellow());
        println!("   Delegating to captain for license registration...");
        self.delegate_to_captain(vec!["license", "register", license_key])?;
        Ok(())
    }
    pub fn get_stored_license_info(&self) -> Result<(String, String)> {
        println!(
            "🔐 {}", "Retrieving license information requires captain binary"
            .bright_blue()
        );
        println!("   Delegating to captain for license info...");
        self.delegate_to_captain(vec!["license", "info"])
            .map(|_| ("CM-DEMO-KEY".to_string(), "FREE".to_string()))
    }
    pub fn get_or_create_user_id(&self) -> Result<String> {
        println!("👤 {}", "User ID management requires captain binary".bright_blue());
        println!("   Delegating to captain for user ID...");
        self.delegate_to_captain(vec!["license", "userid"])
            .map(|_| "user_demo_123".to_string())
    }
    fn delegate_to_captain(&self, args: Vec<&str>) -> Result<bool> {
        let captain_path = match crate::captain::captain_status::find_captain_binary() {
            Some(path) => path,
            None => {
                println!("❌ {}", "Advanced captain binary not found".red().bold());
                println!(
                    "🔄 {}", "Auto-downloading captain binary from get.cargo.do/"
                    .cyan()
                );
                match crate::captain::captain_status::auto_download_captain() {
                    Ok(path) => path,
                    Err(e) => {
                        println!(
                            "❌ {}", format!("Failed to download captain: {}", e) .red()
                        );
                        println!("💡 {}", "Please run: cm captain install".cyan());
                        println!("   Or visit: https://cargo.do/pro");
                        println!();
                        println!(
                            "💡 {}", "License features require the captain binary:"
                            .cyan()
                        );
                        println!("   • License validation and enforcement");
                        println!("   • User ID management");
                        println!("   • Pro feature access");
                        println!("   • Advanced configuration");
                        return Ok(true);
                    }
                }
            }
        };
        let output = Command::new(&captain_path)
            .args(&args)
            .output()
            .context("Failed to execute captain binary for license operation")?;
        if !output.stdout.is_empty() {
            print!("{}", String::from_utf8_lossy(& output.stdout));
        }
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(& output.stderr));
        }
        if !output.status.success() {
            println!(
                "❌ {}", format!("Captain binary exited with status: {}", output.status)
                .red()
            );
        }
        Ok(output.status.success())
    }
}