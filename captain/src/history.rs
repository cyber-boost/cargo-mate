use crate::parser::{ParsedError, ParsedWarning};
use chrono::{DateTime, Utc};
use colored::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use anyhow::Result;
#[derive(Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub error_count: usize,
    pub warning_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
pub fn save_to_history(
    command: String,
    errors: Vec<ParsedError>,
    warnings: Vec<ParsedWarning>,
) {
    let history_file = match get_history_file() {
        Ok(file) => file,
        Err(_) => return,
    };
    let entry = HistoryEntry {
        timestamp: Utc::now(),
        command,
        error_count: errors.len(),
        warning_count: warnings.len(),
        errors: errors.iter().map(|e| e.to_string()).collect(),
        warnings: warnings.iter().map(|w| w.to_string()).collect(),
    };
    let mut history = load_history();
    history.push(entry);
    if history.len() > 1000 {
        let skip_count = history.len() - 1000;
        history = history.into_iter().skip(skip_count).collect();
    }
    if let Some(parent) = history_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(file) = fs::File::create(&history_file) {
        let writer = BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(writer, &history);
    }
}
pub fn show_history(args: &[String]) {
    let history = load_history();
    if history.is_empty() {
        println!("No history found.");
        return;
    }
    let (history_type, limit) = parse_history_args(args);
    match history_type.as_str() {
        "errors" => show_errors(&history, limit),
        "warnings" => show_warnings(&history, limit),
        "summary" => show_summary(&history, limit),
        _ => show_summary(&history, limit),
    }
}
fn parse_history_args(args: &[String]) -> (String, usize) {
    let history_type = args
        .get(0)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "summary".to_string());
    let limit = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(50);
    (history_type, limit)
}
fn show_errors(history: &[HistoryEntry], limit: usize) {
    println!("{}", "=== Error History ===".red().bold());
    let mut all_errors = Vec::new();
    for entry in history.iter().rev() {
        for error in &entry.errors {
            all_errors.push((entry.timestamp, error.clone()));
            if all_errors.len() >= limit {
                break;
            }
        }
        if all_errors.len() >= limit {
            break;
        }
    }
    for (timestamp, error) in all_errors {
        println!(
            "{} - {}", timestamp.format("%Y-%m-%d %H:%M:%S").to_string().dimmed(), error
        );
    }
}
fn show_warnings(history: &[HistoryEntry], limit: usize) {
    println!("{}", "=== Warning History ===".yellow().bold());
    let mut all_warnings = Vec::new();
    for entry in history.iter().rev() {
        for warning in &entry.warnings {
            all_warnings.push((entry.timestamp, warning.clone()));
            if all_warnings.len() >= limit {
                break;
            }
        }
        if all_warnings.len() >= limit {
            break;
        }
    }
    for (timestamp, warning) in all_warnings {
        println!(
            "{} - {}", timestamp.format("%Y-%m-%d %H:%M:%S").to_string().dimmed(),
            warning
        );
    }
}
fn show_summary(history: &[HistoryEntry], limit: usize) {
    println!("{}", "=== Build History Summary ===".blue().bold());
    let recent: Vec<_> = history.iter().rev().take(limit).collect();
    let total_builds = recent.len();
    let successful_builds = recent.iter().filter(|e| e.error_count == 0).count();
    let failed_builds = total_builds - successful_builds;
    println!("📊 Last {} builds:", total_builds);
    println!("  ✅ Successful: {}", successful_builds.to_string().green());
    println!("  ❌ Failed: {}", failed_builds.to_string().red());
    println!("\n📈 Recent builds:");
    for entry in recent.iter().take(10) {
        let status = if entry.error_count == 0 {
            "✅".to_string()
        } else {
            "❌".to_string()
        };
        println!(
            "  {} {} - {} - 🔴 {} ⚠️ {}", status, entry.timestamp
            .format("%Y-%m-%d %H:%M:%S").to_string().dimmed(), entry.command.cyan(),
            entry.error_count, entry.warning_count
        );
    }
}
fn load_history() -> Vec<HistoryEntry> {
    let history_file = match get_history_file() {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };
    if !history_file.exists() {
        return Vec::new();
    }
    match fs::File::open(&history_file) {
        Ok(file) => {
            let reader = BufReader::new(file);
            serde_json::from_reader(reader).unwrap_or_else(|_| Vec::new())
        }
        Err(_) => Vec::new(),
    }
}
fn get_history_file() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not get home directory"))?;
    let shipwreck = home.join(".shipwreck");
    Ok(shipwreck.join("history").join("history.json"))
}