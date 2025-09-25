use anyhow::Result;
use colored::*;
use crate::captain::captain_log;
use crate::cmd::smune::LogAction;
pub fn handle_log(action: LogAction) -> Result<()> {
    let mut log = crate::captain::captain_log::CaptainLog::new()?;
    match action {
        LogAction::Add { message, tags } => {
            log.log(&message, tags)?;
        }
        LogAction::Search { query } => {
            let results = log.search(&query);
            if results.is_empty() {
                println!("No matching log entries found");
            } else {
                println!("Found {} entries:", results.len());
                for entry in results {
                    println!(
                        "  {} - {}", entry.timestamp.format("%Y-%m-%d %H:%M:%S"), entry
                        .message
                    );
                }
            }
        }
        LogAction::Timeline { days } => {
            let days = days.to_owned().try_into().unwrap();
            log.show_timeline(days)?;
        }
        LogAction::Export { path, format } => {
            let fmt = match format.as_str() {
                "json" => captain_log::ExportFormat::Json,
                "html" => captain_log::ExportFormat::Html,
                _ => captain_log::ExportFormat::Markdown,
            };
            log.export(&path, fmt)?;
        }
        LogAction::Analyze => {
            let analysis = log.analyze();
            analysis.display();
        }
        LogAction::Track { command } => {
            println!("🔍 Starting enhanced tracking for: {}", command.cyan());
            let session_id = format!(
                "{}-{:x}", command.replace(" ", "_"), chrono::Utc::now().timestamp()
            );
            match crate::cmd::utils::run_tracked_command(&command, &session_id) {
                Ok(_) => println!("✅ Command tracked successfully"),
                Err(e) => eprintln!("❌ Tracking failed: {}", e),
            }
        }
    }
    Ok(())
}