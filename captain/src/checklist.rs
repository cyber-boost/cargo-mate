use crate::parser::{ParsedError, ParsedWarning};
use chrono::Utc;
use colored::*;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
pub fn generate_checklist(errors: &[ParsedError], warnings: &[ParsedWarning]) {
    let checklist_file = get_checklist_file();
    let mut content = String::new();
    content
        .push_str(
            &format!(
                "=== Build Checklist [{} errors, {} warnings] ===\n", errors.len(),
                warnings.len()
            ),
        );
    content
        .push_str(&format!("Generated: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S")));
    if !errors.is_empty() {
        content.push_str("ERRORS (must fix):\n");
        for error in errors {
            content
                .push_str(
                    &format!(
                        "[ ] Fix {} in {}:{} - {}\n", error.code, error.file, error.line,
                        error.message
                    ),
                );
        }
        content.push_str("\n");
    }
    if !warnings.is_empty() {
        content.push_str("WARNINGS (consider fixing):\n");
        for warning in warnings {
            content
                .push_str(
                    &format!(
                        "[ ] {} in {}:{} - {}\n", warning.code, warning.file, warning
                        .line, warning.message
                    ),
                );
        }
    }
    let mut file = fs::File::create(&checklist_file).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let archive_file = get_checklist_dir().join(format!("checklist_{}.txt", timestamp));
    fs::copy(&checklist_file, &archive_file).unwrap();
}
pub fn show_checklist() {
    let checklist_file = get_checklist_file();
    if !checklist_file.exists() {
        println!("No checklist found. Run a cargo command first!");
        return;
    }
    let content = fs::read_to_string(&checklist_file).unwrap();
    for line in content.lines() {
        if line.starts_with("===") {
            println!("{}", line.blue().bold());
        } else if line.starts_with("ERRORS") {
            println!("{}", line.red().bold());
        } else if line.starts_with("WARNINGS") {
            println!("{}", line.yellow().bold());
        } else if line.starts_with("[ ]") {
            if line.contains("Fix") {
                println!("{}", line.red());
            } else {
                println!("{}", line.yellow());
            }
        } else {
            println!("{}", line);
        }
    }
    println!("\n💡 Tip: Copy this checklist to your editor to track progress!");
}
fn get_checklist_file() -> PathBuf {
    let shipwreck = dirs::home_dir().unwrap().join(".shipwreck");
    shipwreck.join("checklists").join("latest.txt")
}
fn get_checklist_dir() -> PathBuf {
    let shipwreck = dirs::home_dir().unwrap().join(".shipwreck");
    shipwreck.join("checklists")
}