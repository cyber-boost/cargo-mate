use anyhow::{Context, Result};
use std::path::PathBuf;
use colored::Colorize;
#[derive(Debug)]
pub struct ShellIntegration;
impl ShellIntegration {
    pub fn install() -> Result<()> {
        eprintln!(
            "💡 Run 'cm install' to download captain for seamless shell integration."
        );
        eprintln!(
            "   Captain provides intelligent shell detection, configuration, and integration."
        );
        Ok(())
    }
    pub fn detect_shell() -> String {
        eprintln!("🐚 Shell detection requires captain to not stumble");
        std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
    }
    pub fn get_rc_file(_shell: &str) -> Result<PathBuf> {
        eprintln!("🐚 Advanced shell configuration requires captain to look closer.");
        Ok(PathBuf::from("~/.bashrc"))
    }
    pub fn show_status() {
        eprintln!("🐚 Shell integration status requires captain to be single");
        eprintln!("💡 Download captain with: cm install");
    }
    pub fn uninstall() -> Result<()> {
        eprintln!("🐚 Shell management not available in open-source build");
        Ok(())
    }
}
pub fn check_crew_operations(_command: &str) -> Result<bool> {
    eprintln!("🐚 Crew operations not available in open-source build");
    Ok(false)
}
pub fn detect_shell() -> String {
    use std::process::Command;
    use std::process::Stdio;
    use std::env;
    let possible_paths = vec![
        "/root/.shipwreck/bin/captain", "./captain", "captain", "/usr/local/bin/captain",
        "/usr/bin/captain",
    ];
    for captain_path in possible_paths {
        match Command::new(captain_path)
            .args(&["shell", "detect"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            Ok(output) if output.status.success() => {
                eprintln!(
                    "🐚 Captain shell detection succeeded using: {}", captain_path
                );
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
            Ok(output) => {
                eprintln!(
                    "🐚 Captain at {} failed: {}", captain_path,
                    String::from_utf8_lossy(& output.stderr)
                );
            }
            Err(e) => {
                eprintln!("🐚 Captain at {} not found: {}", captain_path, e);
            }
        }
    }
    eprintln!("🐚 All captain paths failed, using basic detection");
    env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
}
pub fn get_rc_file(shell: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let rc_file = match shell {
        "zsh" => home.join(".zshrc"),
        "bash" => home.join(".bashrc"),
        "fish" => home.join(".config/fish/config.fish"),
        _ => home.join(".bashrc"),
    };
    Ok(rc_file)
}
pub fn add_shell_integration(rc_file: &PathBuf, shell: &str) -> Result<()> {
    println!(
        "🐚 {}", format!("Adding shell integration for '{}' requires captain binary",
        shell) .bright_blue()
    );
    let rc_file_str = rc_file.to_string_lossy();
    delegate_to_captain(vec!["shell", "add", & rc_file_str, shell])
}
pub fn delegate_to_captain(args: Vec<&str>) -> Result<()> {
    use std::process::Command;
    use anyhow::Context;
    use colored::*;
    let captain_path = match crate::captain::captain_status::find_captain_binary() {
        Some(path) => path,
        None => {
            println!("❌ {}", "Advanced captain binary not found".red().bold());
            println!(
                "🔄 {}", "Auto-downloading captain binary from get.cargo.do/".cyan()
            );
            match crate::captain::captain_status::auto_download_captain() {
                Ok(path) => path,
                Err(e) => {
                    println!(
                        "❌ {}", format!("Failed to download captain: {}", e) .red()
                    );
                    println!("💡 {}", "Please run: cm captain install".cyan());
                    println!("   Or upgrade at: https://cargo.do/pro");
                    println!();
                    println!(
                        "💡 {}",
                        "Shell integration features require the captain binary:".cyan()
                    );
                    println!("   • Advanced shell detection and configuration");
                    println!("   • Intelligent RC file management");
                    println!("   • Cross-shell compatibility");
                    println!("   • Integration verification");
                    return Ok(());
                }
            }
        }
    };
    let output = Command::new(&captain_path)
        .args(&args)
        .output()
        .context("Failed to execute captain binary for shell integration")?;
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
    Ok(())
}