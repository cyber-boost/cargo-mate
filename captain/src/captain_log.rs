use anyhow::{Context, Result};
use chrono::{DateTime, Utc, TimeDelta};
use colored::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, HashSet};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use crate::captain::license;
use crate::parser::ParsedError;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CargoMessage {
    pub reason: String,
    pub message: Option<CargoDiagnostic>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CargoDiagnostic {
    pub message: String,
    pub code: Option<CargoErrorCode>,
    pub level: String,
    pub spans: Vec<CargoSpan>,
    pub children: Vec<CargoDiagnostic>,
    pub rendered: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CargoErrorCode {
    pub code: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CargoSpan {
    pub file_name: String,
    pub line_start: u32,
    pub line_end: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub text: Vec<CargoText>,
    pub suggested_replacement: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CargoText {
    pub text: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PatternCache {
    recent_sessions: VecDeque<SessionData>,
    max_sessions: usize,
    error_lifecycles: HashMap<String, ErrorLifecycle>,
    fix_patterns: HashMap<String, Vec<FixPattern>>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub command: String,
    pub errors: Vec<ParsedError>,
    pub warnings: Vec<ParsedError>,
    pub success: bool,
    pub duration: Duration,
    pub files_changed: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorLifecycle {
    fingerprint: String,
    first_seen: DateTime<Utc>,
    last_seen: DateTime<Utc>,
    appearances: Vec<SessionAppearance>,
    resolution: Option<Resolution>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionAppearance {
    session_id: String,
    timestamp: DateTime<Utc>,
    count: usize,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Resolution {
    session_id: String,
    commands_between: Vec<String>,
    files_changed: Vec<String>,
    time_to_fix: Duration,
    stayed_fixed: bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FixPattern {
    suggestion: String,
    success_count: usize,
    failure_count: usize,
    last_used: DateTime<Utc>,
}
#[derive(Debug, Clone)]
pub struct BuildImpact {
    pub likely_errors: Vec<String>,
    pub estimated_duration: f64,
    pub affected_files: Vec<String>,
}
impl Default for BuildImpact {
    fn default() -> BuildImpact {
        BuildImpact {
            likely_errors: Vec::new(),
            estimated_duration: 0.0,
            affected_files: Vec::new(),
        }
    }
}
impl PatternCache {
    pub fn new() -> Result<Self> {
        let cache_file = dirs::home_dir()
            .unwrap()
            .join(".shipwreck")
            .join("pattern_cache.json");
        if cache_file.exists() {
            let content = fs::read_to_string(&cache_file)?;
            Ok(serde_json::from_str(&content).unwrap_or_default())
        } else {
            Ok(Self {
                recent_sessions: VecDeque::with_capacity(50),
                max_sessions: 50,
                error_lifecycles: HashMap::new(),
                fix_patterns: HashMap::new(),
            })
        }
    }
    pub fn learn_from_session(&mut self, session: SessionData) -> Result<()> {
        let prev_session = self.recent_sessions.back().cloned();
        if let Some(prev_session) = prev_session {
            self.detect_resolutions(&prev_session, &session);
        }
        for error in &session.errors {
            let fingerprint = self.fingerprint(error);
            self.error_lifecycles
                .entry(fingerprint.clone())
                .or_insert_with(|| ErrorLifecycle::new(fingerprint))
                .add_appearance(&session.id);
        }
        self.recent_sessions.push_back(session);
        while self.recent_sessions.len() > self.max_sessions {
            self.recent_sessions.pop_front();
        }
        self.save()?;
        Ok(())
    }
    pub fn suggest_fix(&self, error: &ParsedError) -> Option<String> {
        let fingerprint = self.fingerprint(error);
        if let Some(patterns) = self.fix_patterns.get(&fingerprint) {
            patterns.iter().max_by_key(|p| p.success_count).map(|p| p.suggestion.clone())
        } else {
            self.find_similar_fix(&fingerprint)
        }
    }
    pub fn predict_build_impact(&self, files_changed: &[String]) -> BuildImpact {
        let mut impact = BuildImpact::default();
        for file in files_changed {
            for session in &self.recent_sessions {
                if session.files_changed.contains(file) {
                    impact.add_historical_data(&session);
                }
            }
        }
        impact
    }
    pub fn calculate_project_health(&self) -> ProjectHealth {
        let total_sessions = self.recent_sessions.len();
        let successful_sessions = self
            .recent_sessions
            .iter()
            .filter(|s| s.success)
            .count();
        let success_rate = if total_sessions > 0 {
            (successful_sessions as f64 / total_sessions as f64) * 100.0
        } else {
            0.0
        };
        let success_rate_trend = if total_sessions >= 10 {
            let recent = &self.recent_sessions.iter().rev().take(5).collect::<Vec<_>>();
            let older = &self
                .recent_sessions
                .iter()
                .rev()
                .skip(5)
                .take(5)
                .collect::<Vec<_>>();
            let recent_rate = recent.iter().filter(|s| s.success).count() as f64
                / recent.len() as f64;
            let older_rate = older.iter().filter(|s| s.success).count() as f64
                / older.len() as f64;
            recent_rate - older_rate
        } else {
            0.0
        };
        let errors_per_day = if total_sessions > 0 {
            let total_errors: usize = self
                .recent_sessions
                .iter()
                .map(|s| s.errors.len())
                .sum();
            let days = self.recent_sessions.len() as f64 / 24.0;
            total_errors as f64 / days.max(1.0)
        } else {
            0.0
        };
        let avg_errors_per_day = if self.recent_sessions.len() > 10 {
            let mid_point = self.recent_sessions.len() / 2;
            let first_half = &self
                .recent_sessions
                .iter()
                .take(mid_point)
                .collect::<Vec<_>>();
            let second_half = &self
                .recent_sessions
                .iter()
                .skip(mid_point)
                .collect::<Vec<_>>();
            let first_avg = first_half.iter().map(|s| s.errors.len()).sum::<usize>()
                as f64 / first_half.len() as f64;
            let second_avg = second_half.iter().map(|s| s.errors.len()).sum::<usize>()
                as f64 / second_half.len() as f64;
            (first_avg + second_avg) / 2.0
        } else {
            errors_per_day
        };
        let avg_time_to_fix = if !self.error_lifecycles.is_empty() {
            let total_fix_time: Duration = self
                .error_lifecycles
                .values()
                .filter_map(|lc| lc.resolution.as_ref())
                .map(|r| r.time_to_fix)
                .sum();
            total_fix_time / self.error_lifecycles.len() as u32
        } else {
            Duration::ZERO
        };
        let top_error_hotspot = self.find_most_problematic_file();
        ProjectHealth {
            current_success_rate: success_rate,
            success_rate_trend,
            errors_per_day,
            avg_errors_per_day,
            avg_time_to_fix,
            top_error_hotspot,
        }
    }
    fn detect_resolutions(
        &mut self,
        prev_session: &SessionData,
        current_session: &SessionData,
    ) {
        if prev_session.success || current_session.errors.is_empty() {
            return;
        }
        let current_fingerprints: HashSet<String> = current_session
            .errors
            .iter()
            .map(|e| self.fingerprint(e))
            .collect();
        for error in &prev_session.errors {
            let fingerprint = self.fingerprint(error);
            if let Some(lifecycle) = self.error_lifecycles.get_mut(&fingerprint) {
                if !current_fingerprints.contains(&fingerprint) {
                    let resolution = Resolution {
                        session_id: current_session.id.clone(),
                        commands_between: vec![],
                        files_changed: current_session.files_changed.clone(),
                        time_to_fix: current_session
                            .timestamp
                            .signed_duration_since(prev_session.timestamp)
                            .to_std()
                            .unwrap_or(Duration::ZERO),
                        stayed_fixed: true,
                    };
                    lifecycle.resolution = Some(resolution);
                }
            }
        }
    }
    fn fingerprint(&self, error: &ParsedError) -> String {
        format!(
            "{}:{}", error.code, error.message.chars().take(50).collect::< String > ()
        )
    }
    fn find_similar_fix(&self, fingerprint: &str) -> Option<String> {
        for (pattern_fp, patterns) in &self.fix_patterns {
            if pattern_fp.contains(&fingerprint[0..10.min(fingerprint.len())]) {
                return patterns
                    .iter()
                    .max_by_key(|p| p.success_count)
                    .map(|p| p.suggestion.clone());
            }
        }
        None
    }
    fn find_most_problematic_file(&self) -> Option<ErrorHotspot> {
        let mut file_errors = HashMap::new();
        for session in &self.recent_sessions {
            for error in &session.errors {
                if !error.file.is_empty() {
                    *file_errors.entry(error.file.clone()).or_insert(0) += 1;
                }
            }
        }
        file_errors
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(file, error_count)| ErrorHotspot { file, error_count })
    }
    fn save(&self) -> Result<()> {
        let cache_file = dirs::home_dir()
            .unwrap()
            .join(".shipwreck")
            .join("pattern_cache.json");
        fs::create_dir_all(cache_file.parent().unwrap())?;
        let json = serde_json::to_string_pretty(self)?;
        fs::write(cache_file, json)?;
        Ok(())
    }
}
impl Default for PatternCache {
    fn default() -> Self {
        Self {
            recent_sessions: VecDeque::with_capacity(50),
            max_sessions: 50,
            error_lifecycles: HashMap::new(),
            fix_patterns: HashMap::new(),
        }
    }
}
impl ErrorLifecycle {
    fn new(fingerprint: String) -> Self {
        Self {
            fingerprint,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
            appearances: Vec::new(),
            resolution: None,
        }
    }
    fn add_appearance(&mut self, session_id: &str) {
        self.appearances
            .push(SessionAppearance {
                session_id: session_id.to_string(),
                timestamp: Utc::now(),
                count: 1,
            });
        self.last_seen = Utc::now();
    }
}
impl BuildImpact {
    fn add_historical_data(&mut self, session: &SessionData) {
        for error in &session.errors {
            self.likely_errors.push(error.message.clone());
        }
        self.estimated_duration += session.duration.as_secs_f64();
        self.affected_files.extend(session.files_changed.clone());
    }
}
#[derive(Debug)]
pub struct ProjectHealth {
    pub current_success_rate: f64,
    pub success_rate_trend: f64,
    pub errors_per_day: f64,
    pub avg_errors_per_day: f64,
    pub avg_time_to_fix: Duration,
    pub top_error_hotspot: Option<ErrorHotspot>,
}
#[derive(Debug)]
pub struct ErrorHotspot {
    pub file: String,
    pub error_count: usize,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub tags: Vec<String>,
    pub command: Option<String>,
    pub build_result: Option<BuildResult>,
    pub context: HashMap<String, String>,
    pub error_code: Option<String>,
    pub error_type: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
    pub suggestion: Option<String>,
    pub full_diagnostic: Option<serde_json::Value>,
    pub resolved_in_session: Option<String>,
    pub warning_type: Option<String>,
    pub lint_name: Option<String>,
    pub severity: Option<String>,
    pub suppressed: Option<bool>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildResult {
    pub success: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub duration_seconds: f64,
}
pub struct CaptainLog {
    entries: Vec<LogEntry>,
    current_session: Vec<LogEntry>,
    log_file: PathBuf,
}
impl CaptainLog {
    pub fn new() -> Result<Self> {
        let shipwreck_dir = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck");
        fs::create_dir_all(&shipwreck_dir)?;
        let log_file = shipwreck_dir.join("captain.log");
        let entries = if log_file.exists() {
            let content = fs::read_to_string(&log_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(Self {
            entries,
            current_session: Vec::new(),
            log_file,
        })
    }
    pub fn log(&mut self, message: &str, tags: Vec<String>) -> Result<()> {
        let entry = LogEntry {
            timestamp: Utc::now(),
            message: message.to_string(),
            tags,
            command: None,
            build_result: None,
            context: self.capture_context(),
            error_code: None,
            error_type: None,
            file_path: None,
            line_number: None,
            column_number: None,
            suggestion: None,
            full_diagnostic: None,
            resolved_in_session: None,
            warning_type: None,
            lint_name: None,
            severity: None,
            suppressed: None,
        };
        self.entries.push(entry.clone());
        self.current_session.push(entry.clone());
        self.save()?;
        println!("📝 {}", format!("Logged: {}", message) .green());
        if !entry.tags.is_empty() {
            println!("   🏷️  Tags: {}", entry.tags.join(", ").dimmed());
        }
        Ok(())
    }
    pub fn log_command(&mut self, command: &str, result: BuildResult) -> Result<()> {
        let entry = LogEntry {
            timestamp: Utc::now(),
            message: format!("Executed: {}", command),
            tags: vec!["command".to_string()],
            command: Some(command.to_string()),
            build_result: Some(result.clone()),
            context: self.capture_context(),
            error_code: None,
            error_type: None,
            file_path: None,
            line_number: None,
            column_number: None,
            suggestion: None,
            full_diagnostic: None,
            resolved_in_session: None,
            warning_type: None,
            lint_name: None,
            severity: None,
            suppressed: None,
        };
        self.entries.push(entry.clone());
        self.current_session.push(entry);
        self.save()?;
        let status_icon = if result.success { "✅" } else { "❌" };
        println!(
            "{} Command logged: {} ({}s)", status_icon, command.cyan(), result
            .duration_seconds
        );
        Ok(())
    }
    pub fn search(&self, query: &str) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                entry.message.to_lowercase().contains(&query.to_lowercase())
                    || entry
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query.to_lowercase()))
            })
            .collect()
    }
    pub fn search_by_tag(&self, tag: &str) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|entry| { entry.tags.iter().any(|t| t == tag) })
            .collect()
    }
    pub fn get_recent(&self, count: usize) -> Vec<&LogEntry> {
        let start = if self.entries.len() > count {
            self.entries.len() - count
        } else {
            0
        };
        self.entries[start..].iter().collect()
    }
    pub fn get_session_logs(&self) -> &[LogEntry] {
        &self.current_session
    }
    pub fn show_timeline(&self, days: i64) -> Result<()> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        let filtered: Vec<&LogEntry> = self
            .entries
            .iter()
            .filter(|entry| entry.timestamp > cutoff)
            .collect();
        if filtered.is_empty() {
            println!("No log entries in the last {} days", days);
            return Ok(());
        }
        println!(
            "{}", format!("=== Captain's Log - Last {} Days ===", days) .blue().bold()
        );
        let mut current_date = None;
        for entry in filtered {
            let entry_date = entry.timestamp.date_naive();
            if current_date != Some(entry_date) {
                println!(
                    "\n📅 {}", entry_date.format("%A, %B %d, %Y").to_string().yellow()
                );
                current_date = Some(entry_date);
            }
            let time = entry.timestamp.format("%H:%M:%S");
            let icon = if entry.command.is_some() { "⚙️" } else { "📝" };
            print!("  {} {} - ", icon, time.to_string().dimmed());
            if let Some(ref result) = entry.build_result {
                let status = if result.success { "✅" } else { "❌" };
                print!("{} ", status);
            }
            println!("{}", entry.message);
            if !entry.tags.is_empty() && entry.tags != vec!["command"] {
                println!("      🏷️  {}", entry.tags.join(", ").dimmed());
            }
        }
        Ok(())
    }
    pub fn export(&self, path: &PathBuf, format: ExportFormat) -> Result<()> {
        match format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&self.entries)?;
                fs::write(path, json)?;
            }
            ExportFormat::Markdown => {
                let mut content = String::new();
                content.push_str("# Captain's Log\n\n");
                for entry in &self.entries {
                    content
                        .push_str(
                            &format!(
                                "## {}\n", entry.timestamp.format("%Y-%m-%d %H:%M:%S")
                            ),
                        );
                    content.push_str(&format!("\n{}\n", entry.message));
                    if !entry.tags.is_empty() {
                        content
                            .push_str(
                                &format!("\n**Tags:** {}\n", entry.tags.join(", ")),
                            );
                    }
                    if let Some(ref cmd) = entry.command {
                        content.push_str(&format!("\n**Command:** `{}`\n", cmd));
                    }
                    if let Some(ref result) = entry.build_result {
                        content
                            .push_str(
                                &format!(
                                    "\n**Result:** {} ({} errors, {} warnings, {:.2}s)\n", if
                                    result.success { "✅ Success" } else { "❌ Failed" },
                                    result.error_count, result.warning_count, result
                                    .duration_seconds
                                ),
                            );
                    }
                    content.push_str("\n---\n\n");
                }
                fs::write(path, content)?;
            }
            ExportFormat::Html => {
                let mut content = String::new();
                content
                    .push_str(
                        r#"<!DOCTYPE html>
<html>
<head>
    <title>Captain's Log</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
               max-width: 900px; margin: 0 auto; padding: 20px; background: #f5f5f5; }
        .entry { background: white; padding: 15px; margin: 10px 0; border-radius: 8px; 
                 box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .timestamp { color: #666; font-size: 0.9em; }
        .message { margin: 10px 0; font-size: 1.1em; }
        .tags { display: inline-block; background: #e0e0e0; padding: 3px 8px; 
                border-radius: 3px; margin: 2px; font-size: 0.85em; }
        .success { color: green; }
        .failure { color: red; }
        .command { font-family: monospace; background: #f0f0f0; padding: 5px; 
                   border-radius: 3px; }
    </style>
</head>
<body>
    <h1>⚓ Captain's Log</h1>
"#,
                    );
                for entry in &self.entries {
                    content.push_str("<div class='entry'>");
                    content
                        .push_str(
                            &format!(
                                "<div class='timestamp'>{}</div>", entry.timestamp
                                .format("%Y-%m-%d %H:%M:%S")
                            ),
                        );
                    content
                        .push_str(
                            &format!("<div class='message'>{}</div>", entry.message),
                        );
                    if !entry.tags.is_empty() {
                        content.push_str("<div>");
                        for tag in &entry.tags {
                            content
                                .push_str(&format!("<span class='tags'>{}</span>", tag));
                        }
                        content.push_str("</div>");
                    }
                    if let Some(ref cmd) = entry.command {
                        content.push_str(&format!("<div class='command'>{}</div>", cmd));
                    }
                    if let Some(ref result) = entry.build_result {
                        let class = if result.success { "success" } else { "failure" };
                        content
                            .push_str(
                                &format!(
                                    "<div class='{}'>{} - {} errors, {} warnings ({:.2}s)</div>",
                                    class, if result.success { "✅ Success" } else {
                                    "❌ Failed" }, result.error_count, result.warning_count,
                                    result.duration_seconds
                                ),
                            );
                    }
                    content.push_str("</div>");
                }
                content.push_str("</body></html>");
                fs::write(path, content)?;
            }
        }
        println!("✅ Log exported to {}", path.display());
        Ok(())
    }
    pub fn analyze(&self) -> LogAnalysis {
        let total_entries = self.entries.len();
        let commands: Vec<&LogEntry> = self
            .entries
            .iter()
            .filter(|e| e.command.is_some())
            .collect();
        let total_commands = commands.len();
        let successful_builds = commands
            .iter()
            .filter(|e| e.build_result.as_ref().map_or(false, |r| r.success))
            .count();
        let failed_builds = total_commands - successful_builds;
        let avg_build_time = if !commands.is_empty() {
            let total_time: f64 = commands
                .iter()
                .filter_map(|e| e.build_result.as_ref())
                .map(|r| r.duration_seconds)
                .sum();
            total_time / commands.len() as f64
        } else {
            0.0
        };
        let mut tag_frequency = HashMap::new();
        for entry in &self.entries {
            for tag in &entry.tags {
                *tag_frequency.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let mut most_common_tags: Vec<(String, usize)> = tag_frequency
            .into_iter()
            .collect();
        most_common_tags.sort_by(|a, b| b.1.cmp(&a.1));
        most_common_tags.truncate(5);
        LogAnalysis {
            total_entries,
            total_commands,
            successful_builds,
            failed_builds,
            success_rate: if total_commands > 0 {
                (successful_builds as f64 / total_commands as f64) * 100.0
            } else {
                0.0
            },
            avg_build_time,
            most_common_tags,
        }
    }
    fn capture_context(&self) -> HashMap<String, String> {
        let mut context = HashMap::new();
        if let Ok(dir) = std::env::current_dir() {
            context.insert("working_dir".to_string(), dir.to_string_lossy().to_string());
        }
        if let Ok(branch) = get_git_branch() {
            context.insert("git_branch".to_string(), branch);
        }
        context
    }
    fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.log_file, json)?;
        Ok(())
    }
}
#[derive(Debug)]
pub struct LogAnalysis {
    pub total_entries: usize,
    pub total_commands: usize,
    pub successful_builds: usize,
    pub failed_builds: usize,
    pub success_rate: f64,
    pub avg_build_time: f64,
    pub most_common_tags: Vec<(String, usize)>,
}
impl LogAnalysis {
    pub fn display(&self) {
        println!("{}", "=== Captain's Log Analysis ===".blue().bold());
        println!("📊 Total entries: {}", self.total_entries);
        println!("⚙️  Total commands: {}", self.total_commands);
        println!(
            "✅ Successful builds: {}", self.successful_builds.to_string().green()
        );
        println!("❌ Failed builds: {}", self.failed_builds.to_string().red());
        println!("📈 Success rate: {:.1}%", self.success_rate);
        println!("⏱️  Average build time: {:.2}s", self.avg_build_time);
        if !self.most_common_tags.is_empty() {
            println!("\n🏷️  Most common tags:");
            for (tag, count) in &self.most_common_tags {
                println!("   {} ({})", tag.cyan(), count);
            }
        }
    }
}
#[derive(Debug)]
pub enum ExportFormat {
    Json,
    Markdown,
    Html,
}
fn get_git_branch() -> Result<String> {
    use std::process::Command;
    let output = Command::new("git").args(&["branch", "--show-current"]).output();
    match output {
        Ok(output) if output.status.success() => {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(branch)
        }
        _ => Ok("unknown".to_string()),
    }
}
pub struct CargoOutputParser;
impl CargoOutputParser {
    pub fn new() -> Self {
        Self
    }
    pub fn parse_message(&self, json_msg: &str) -> Result<Option<CargoMessage>> {
        match serde_json::from_str::<CargoMessage>(json_msg) {
            Ok(msg) => Ok(Some(msg)),
            Err(_) => Ok(None),
        }
    }
    pub fn extract_error_code(&self, diagnostic: &CargoDiagnostic) -> Option<String> {
        diagnostic.code.as_ref().map(|code| code.code.clone())
    }
    pub fn categorize_error(&self, diagnostic: &CargoDiagnostic) -> Option<String> {
        if let Some(code) = &diagnostic.code {
            if code.code.starts_with("E0") {
                Some("type_system".to_string())
            } else if code.code.starts_with("E03") {
                Some("borrow_checker".to_string())
            } else if code.code.starts_with("E04") {
                Some("lifetime".to_string())
            } else if code.code.starts_with("E05") {
                Some("pattern_matching".to_string())
            } else {
                Some("other".to_string())
            }
        } else {
            None
        }
    }
    pub fn extract_suggestions(&self, diagnostic: &CargoDiagnostic) -> Vec<String> {
        let mut suggestions = Vec::new();
        for span in &diagnostic.spans {
            if let Some(replacement) = &span.suggested_replacement {
                suggestions.push(replacement.clone());
            }
        }
        for child in &diagnostic.children {
            suggestions.extend(self.extract_suggestions(child));
        }
        suggestions
    }
    pub fn create_log_entry_from_diagnostic(
        &self,
        diagnostic: &CargoDiagnostic,
        session_id: &str,
    ) -> LogEntry {
        let error_code = self.extract_error_code(diagnostic);
        let error_type = self.categorize_error(diagnostic);
        let suggestions = self.extract_suggestions(diagnostic);
        let (file_path, line_number, column_number) = if !diagnostic.spans.is_empty() {
            let span = &diagnostic.spans[0];
            (
                Some(span.file_name.clone()),
                Some(span.line_start),
                Some(span.column_start),
            )
        } else {
            (None, None, None)
        };
        LogEntry {
            timestamp: Utc::now(),
            message: diagnostic.message.clone(),
            tags: vec![
                diagnostic.level.clone(), error_code.clone().unwrap_or_else(|| "unknown"
                .to_string()),
            ],
            command: Some(session_id.to_string()),
            build_result: None,
            context: HashMap::new(),
            error_code: error_code.clone(),
            error_type: error_type.clone(),
            file_path,
            line_number,
            column_number,
            suggestion: suggestions.first().cloned(),
            full_diagnostic: Some(serde_json::to_value(diagnostic).unwrap_or_default()),
            resolved_in_session: None,
            warning_type: if diagnostic.level == "warning" { error_type } else { None },
            lint_name: if diagnostic.level == "warning" { error_code } else { None },
            severity: Some(diagnostic.level.clone()),
            suppressed: Some(false),
        }
    }
}
pub struct PatternDetector {
    entries: Vec<LogEntry>,
}
impl PatternDetector {
    pub fn new(entries: Vec<LogEntry>) -> Self {
        Self { entries }
    }
    pub fn find_recurring_errors(&self) -> Vec<(String, usize, Vec<String>)> {
        let mut error_counts = HashMap::new();
        for entry in &self.entries {
            if let (Some(error_code), Some(file_path)) = (
                &entry.error_code,
                &entry.file_path,
            ) {
                let key = format!("{}:{}", error_code, file_path);
                error_counts
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(entry.timestamp.to_rfc3339());
            }
        }
        let mut recurring = error_counts
            .into_iter()
            .filter(|(_, timestamps)| timestamps.len() > 2)
            .map(|(key, timestamps)| {
                let parts: Vec<&str> = key.split(':').collect();
                (key, timestamps.len(), timestamps)
            })
            .collect::<Vec<_>>();
        recurring.sort_by(|a, b| b.1.cmp(&a.1));
        recurring
    }
    pub fn detect_build_time_regression(&self) -> Vec<(String, f64, f64)> {
        let mut regressions = Vec::new();
        let mut command_times = HashMap::new();
        for entry in &self.entries {
            if let Some(ref result) = entry.build_result {
                let cmd = entry
                    .command
                    .as_ref()
                    .unwrap_or(&"unknown".to_string())
                    .clone();
                command_times
                    .entry(cmd)
                    .or_insert_with(Vec::new)
                    .push(result.duration_seconds);
            }
        }
        for (command, times) in command_times {
            if times.len() >= 5 {
                let recent_avg = times[times.len().saturating_sub(3)..]
                    .iter()
                    .sum::<f64>() / 3.0;
                let older_avg = times[0..times.len().saturating_sub(3)]
                    .iter()
                    .sum::<f64>() / (times.len().saturating_sub(3)) as f64;
                if recent_avg > older_avg * 1.2 {
                    regressions.push((command, older_avg, recent_avg));
                }
            }
        }
        regressions
    }
    pub fn find_warning_clusters(&self) -> Vec<(String, usize)> {
        let mut file_warnings = HashMap::new();
        for entry in &self.entries {
            if entry.severity.as_ref().map(|s| s == "warning").unwrap_or(false) {
                if let Some(ref file_path) = entry.file_path {
                    *file_warnings.entry(file_path.clone()).or_insert(0) += 1;
                }
            }
        }
        let mut clusters = file_warnings.into_iter().collect::<Vec<_>>();
        clusters.sort_by(|a, b| b.1.cmp(&a.1));
        clusters
    }
}
pub fn check_quartermaster_status(command: &str) -> Result<bool> {
    println!(
        "📋 Quartermaster verifying status for command '{}' - all provisions checked!",
        command.cyan()
    );
    let license_manager = license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => {
            println!(
                "✅ Quartermaster reports: Command '{}' fully provisioned!", command
                .green()
            );
            println!("   📋 All supplies accounted for - ready to execute!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!("⚠️  Quartermaster warning: Supply quota exceeded!");
                println!("   📋 Requisition more at: https://cargo.do/checkout");
                println!("   📋 Upgrade for unlimited command provisions");
            } else if e.to_string().contains("License not found") {
                println!("❌ Quartermaster emergency: No provision authorization!");
                println!("   📋 Get requisition with 'cm register <key>'");
            } else {
                println!(
                    "❌ Quartermaster distress: Status check failed: {}", e.to_string()
                    .red()
                );
                println!("   📋 Secure the manifest - prepare for audit!");
            }
            Ok(false)
        }
    }
}
pub fn show_build_health_dashboard() -> Result<()> {
    let pattern_cache = PatternCache::new().unwrap_or_default();
    let health = pattern_cache.calculate_project_health();
    println!("\n📊 Project Health Dashboard:");
    let trend = if health.success_rate_trend > 0.0 { "📈" } else { "📉" };
    println!(
        "  {} Success Rate: {:.1}% {}", trend, health.current_success_rate,
        format!("({:+.1}%)", health.success_rate_trend) .dimmed()
    );
    let error_trend = if health.errors_per_day < health.avg_errors_per_day {
        "📉"
    } else {
        "📈"
    };
    println!(
        "  {} Error Rate: {:.1}/day {}", error_trend, health.errors_per_day, if health
        .errors_per_day < health.avg_errors_per_day { "(improving)" } else {
        "(worsening)" }
    );
    println!("  ⏱️  Avg Fix Time: {}", format_duration(health.avg_time_to_fix));
    if let Some(hotspot) = health.top_error_hotspot {
        println!("  🔥 Hotspot: {} ({} errors)", hotspot.file, hotspot.error_count);
    }
    if !pattern_cache.recent_sessions.is_empty() {
        println!("\n🎯 Recent Insights:");
        let recent_sessions = pattern_cache.recent_sessions.iter().rev().take(3);
        for session in recent_sessions {
            let status = if session.success { "✅" } else { "❌" };
            println!(
                "  {} {} ({} errors, {:.1}s)", status, session.command, session.errors
                .len(), session.duration.as_secs_f64()
            );
        }
    }
    Ok(())
}
fn format_duration(duration: Duration) -> String {
    let seconds = duration.as_secs_f32() as u64;
    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
    }
}
pub fn detect_changed_files() -> Vec<String> {
    use std::process::Command;
    let output = Command::new("git").args(&["diff", "--name-only", "HEAD"]).output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect()
        }
        _ => vec![],
    }
}
pub fn generate_session_id() -> String {
    format!("session_{}", rand::random::< u64 > ())
}