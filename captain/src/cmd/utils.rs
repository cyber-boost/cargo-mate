use anyhow::{Result, Context};
use colored::*;
use chrono;
use std::fs;
use std::path::PathBuf;
use dirs;
use crate::captain::license;
use crate::captain::wtf;
use crate::captain::version;
use crate::display;
use crate::captain::captain_log;
pub fn run_cargo_with_wrapper(args: &[&str]) -> Result<()> {
    if !args.is_empty() {
        let license_manager = crate::captain::license::LicenseManager::new();
        if let Err(e) = license_manager?.enforce_license(&format!("cargo-{}", args[0])) {
            eprintln!("❌ License enforcement failed: {}", e);
            crate::captain::wtf::display_api_failure_art();
            std::process::exit(1);
        }
    }
    if let Err(e) = version::pre_operation_hook(None) {
        eprintln!("⚠️  Version auto-increment failed: {}", e);
    }
    display::run_cargo_with_display(args);
    if let Ok(mut log) = captain_log::CaptainLog::new() {
        let build_result = captain_log::BuildResult {
            success: true,
            error_count: 0,
            warning_count: 0,
            duration_seconds: 0.0,
        };
        if let Err(e) = log
            .log_command(&format!("cargo {}", args.join(" ")), build_result)
        {
            eprintln!("⚠️  Captain's Log recording failed: {}", e);
        }
        println!("\n📝 {}", "Captain's Log: Session recorded".dimmed());
    }
    if let Err(e) = version::post_operation_hook(None, true) {
        eprintln!("⚠️  Version post-operation hook failed: {}", e);
    }
    Ok(())
}
pub fn run_tracked_command(command: &str, session_id: &str) -> Result<()> {
    use std::process::Command;
    use std::io::{BufRead, BufReader};
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty command"));
    }
    let mut log = captain_log::CaptainLog::new()?;
    let parser = captain_log::CargoOutputParser::new();
    let mut cmd = Command::new(parts[0]);
    cmd.args(&parts[1..]);
    if parts[0] == "cargo" {
        // Only add --message-format=json for commands that support it
        let command_supports_json = parts.get(1).map_or(false, |cmd| {
            matches!(*cmd, "build" | "check" | "test" | "doc" | "clippy" | "fmt")
        });
        if command_supports_json {
            cmd.arg("--message-format=json");
        }

        // Handle cargo publish with automatic version checking
        if parts.get(1) == Some(&"publish") {
            // We can't do the version check here because this function doesn't have access to the right context
            // The version check happens in run_cargo_with_display instead
        }
    }
    let start_time = std::time::Instant::now();
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line?;
            println!("{}", line);
            if let Some(msg) = parser.parse_message(&line)? {
                if let Some(diagnostic) = msg.message {
                    let entry = parser
                        .create_log_entry_from_diagnostic(&diagnostic, session_id);
                    log.log(&entry.message, entry.tags)?;
                }
            }
        }
    }
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let line = line?;
            eprintln!("{}", line);
            if let Some(msg) = parser.parse_message(&line)? {
                if let Some(diagnostic) = msg.message {
                    let entry = parser
                        .create_log_entry_from_diagnostic(&diagnostic, session_id);
                    log.log(&entry.message, entry.tags)?;
                }
            }
        }
    }
    let status = child.wait()?;
    let duration = start_time.elapsed();
    let build_result = captain_log::BuildResult {
        success: status.success(),
        error_count: 0,
        warning_count: 0,
        duration_seconds: duration.as_secs_f64(),
    };
    log.log_command(command, build_result)?;
    println!("\n🔍 Analysis:");
    let entries = log.get_recent(1000);
    let entries_owned: Vec<captain_log::LogEntry> = entries
        .into_iter()
        .cloned()
        .collect();
    let detector = captain_log::PatternDetector::new(entries_owned);
    let recurring = detector.find_recurring_errors();
    if !recurring.is_empty() {
        println!("\n⚠️  Recurring Issues:");
        for (error_key, count, _) in recurring.into_iter().take(5) {
            println!("   {} ({})", error_key.cyan(), count);
        }
    }
    let regressions = detector.detect_build_time_regression();
    if !regressions.is_empty() {
        println!("\n📈 Build Time Regressions:");
        for (command, old_time, new_time) in regressions {
            println!(
                "   {}: {:.2}s → {:.2}s ({:.1}%)", command.cyan(), old_time, new_time,
                ((new_time - old_time) / old_time) * 100.0
            );
        }
    }
    Ok(())
}
pub fn get_recent_errors(count: usize) -> Result<Vec<String>> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let shipwreck = home_dir.join(".shipwreck");
    let error_file = shipwreck.join("errors").join("latest.txt");
    if error_file.exists() {
        let content = std::fs::read_to_string(&error_file)?;
        let errors: Vec<String> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(count)
            .map(|s| s.to_string())
            .collect();
        if errors.is_empty() {
            Ok(vec!["No recent errors found in the error logs.".to_string()])
        } else {
            Ok(errors)
        }
    } else {
        Ok(
            vec![
                "No error log file found. Try running some cargo commands first."
                .to_string()
            ],
        )
    }
}
pub fn show_loading_messages() {
    let messages = [
        "⚓ Hoisting the sails... preparing to set sail for knowledge!",
        "🌊 Riding the waves... surfing through the digital ocean!",
        "🧭 Checking the compass... navigating to the answer!",
        "🚢 Batten down the hatches... stormy seas of computation ahead!",
        "🦜 Arr, matey! Consulting the ancient tomes of wisdom!",
        "⚡ Charging the canons... ready to fire the knowledge salvo!",
        "🧜‍♀️ Singing sea shanties... luring the answers from the deep!",
        "🗺️ Reading the treasure map... X marks the spot of knowledge!",
        "🦈 Dodging digital sharks... swimming towards the answer!",
        "🌟 Aligning the stars... consulting the celestial database!",
    ];
    let mut index = 0;
    let start_time = std::time::Instant::now();
    while start_time.elapsed().as_secs() < 30 {
        println!("⏳ {}", messages[index]);
        std::thread::sleep(std::time::Duration::from_secs(3));
        index = (index + 1) % messages.len();
    }
}
pub fn parse_bool(s: &str) -> Result<bool, std::num::ParseIntError> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Ok(s.parse::<u8>()? != 0),
    }
}
pub fn is_cm_command(cmd: &str) -> bool {
    matches!(
        cmd, "anchor" | "journey" | "log" | "tide" | "map" | "mutiny" | "config" |
        "version" | "view" | "optimize" | "test" | "history" | "init" | "install" |
        "activate" | "register" | "idea" | "wtf" | "checklist" | "add" | "done" | "clear"
        | "show" | "list" | "user" | "debug" | "help" | "--help" | "-h" | "tool" |
        "tools" | "strip" | "scat"
    )
}
pub fn handle_license_check(command: &str) -> Result<()> {
    let license_manager = crate::captain::license::LicenseManager::new();
    license_manager?.enforce_license(command)
}
pub fn check_command_license(command: &str) -> Result<()> {
    let license_manager = crate::captain::license::LicenseManager::new();
    match license_manager?.enforce_license(command) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}