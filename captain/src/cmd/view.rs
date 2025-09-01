use anyhow::{Result, Context};
use std::fs;
use dirs;
use colored::Colorize;
use crate::history;
use crate::checklist;
use crate::cmd::smune::ViewAction;
pub fn handle_view(action: ViewAction) -> Result<()> {
    let shipwreck = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".shipwreck");
    match action {
        ViewAction::Errors => {
            let error_file = shipwreck.join("errors").join("latest.txt");
            if error_file.exists() {
                println!("🔴 Latest Errors:");
                println!("{}", "═".repeat(50).red());
                let content = fs::read_to_string(error_file)?;
                println!("{}", content);
            } else {
                println!("✅ No errors found");
            }
        }
        ViewAction::Artifacts => {
            let artifact_file = shipwreck.join("artifacts").join("latest.txt");
            if artifact_file.exists() {
                println!("📦 Generated Artifacts:");
                println!("{}", "═".repeat(50).blue());
                let content = fs::read_to_string(artifact_file)?;
                println!("{}", content);
            } else {
                println!("📁 No artifacts found");
            }
        }
        ViewAction::Scripts => {
            let script_file = shipwreck.join("scripts").join("latest.txt");
            if script_file.exists() {
                println!("🔨 Build Scripts:");
                println!("{}", "═".repeat(50).yellow());
                let content = fs::read_to_string(script_file)?;
                println!("{}", content);
            }
        }
        ViewAction::History => {
            history::show_history(&["detailed".to_string(), "100".to_string()]);
        }
        ViewAction::Checklist => {
            checklist::show_checklist();
        }
        ViewAction::All => {
            println!("🔍 Complete Build Results:");
            println!("{}", "═".repeat(60).cyan());
            let error_file = shipwreck.join("errors").join("latest.txt");
            if error_file.exists() {
                println!("🔴 Errors:");
                let content = fs::read_to_string(error_file)?;
                println!("{}", content);
                println!();
            }
            let warning_file = shipwreck.join("warnings").join("latest.txt");
            if warning_file.exists() {
                println!("⚠️  Warnings:");
                let content = fs::read_to_string(warning_file)?;
                println!("{}", content);
                println!();
            }
            let artifact_file = shipwreck.join("artifacts").join("latest.txt");
            if artifact_file.exists() {
                println!("📦 Artifacts:");
                let content = fs::read_to_string(artifact_file)?;
                println!("{}", content);
                println!();
            }
            let script_file = shipwreck.join("scripts").join("latest.txt");
            if script_file.exists() {
                println!("🔨 Build Scripts:");
                let content = fs::read_to_string(script_file)?;
                println!("{}", content);
            }
        }
        ViewAction::Latest => {
            println!("🔍 Latest Build Issues:");
            println!("{}", "═".repeat(50).cyan());
            let error_file = shipwreck.join("errors").join("latest.txt");
            if error_file.exists() {
                let content = fs::read_to_string(error_file)?;
                if !content.trim().is_empty() {
                    println!("🔴 Errors:");
                    println!("{}", content);
                    println!();
                }
            }
            let warning_file = shipwreck.join("warnings").join("latest.txt");
            if warning_file.exists() {
                let content = fs::read_to_string(warning_file)?;
                if !content.trim().is_empty() {
                    println!("⚠️  Warnings:");
                    println!("{}", content);
                }
            }
        }
        ViewAction::Open => {
            use std::process::Command;
            let target_dir = std::env::current_dir()?.join("target");
            if target_dir.exists() {
                println!("🚀 Opening target directory in file explorer...");
                #[cfg(target_os = "linux")]
                {
                    let _ = Command::new("xdg-open").arg(&target_dir).spawn();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = Command::new("open").arg(&target_dir).spawn();
                }
                #[cfg(target_os = "windows")]
                {
                    let _ = Command::new("explorer").arg(&target_dir).spawn();
                }
            } else {
                println!("❌ Target directory not found");
            }
        }
    }
    Ok(())
}