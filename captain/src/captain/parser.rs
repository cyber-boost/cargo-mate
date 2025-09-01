use serde::{Deserialize, Serialize};
use std::fmt;
use std::process::Command;
use anyhow::{Context, Result};
use colored::*;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParsedError {
    pub code: String,
    pub file: String,
    pub line: usize,
    pub message: String,
}
impl fmt::Display for ParsedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}:{} - {}", self.code, self.file, self.line, self.message)
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParsedWarning {
    pub code: String,
    pub file: String,
    pub line: usize,
    pub message: String,
}
impl fmt::Display for ParsedWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}:{} - {}", self.code, self.file, self.line, self.message)
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum MessageData {
    CompilerMessage(CompilerMessage),
    BuildScriptExecuted(BuildScriptExecuted),
    CompilerArtifact(CompilerArtifact),
    Other(serde_json::Value),
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompilerMessage {
    pub message: DiagnosticMessage,
    #[serde(default)]
    pub package_id: String,
    pub target: Option<Target>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiagnosticMessage {
    pub message: String,
    pub code: Option<DiagnosticCode>,
    pub level: String,
    pub spans: Vec<DiagnosticSpan>,
    pub children: Vec<DiagnosticMessage>,
    pub rendered: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiagnosticCode {
    pub code: String,
    pub explanation: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiagnosticSpan {
    pub file_name: String,
    pub line_start: usize,
    pub line_end: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub text: Vec<SpanText>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SpanText {
    pub text: String,
    pub highlight_start: usize,
    pub highlight_end: usize,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BuildScriptExecuted {
    pub package_id: String,
    pub linked_libs: Vec<String>,
    pub linked_paths: Vec<String>,
    pub cfgs: Vec<String>,
    pub env: Vec<(String, String)>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompilerArtifact {
    pub package_id: String,
    pub target: Target,
    pub profile: ArtifactProfile,
    pub features: Vec<String>,
    pub filenames: Vec<String>,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Target {
    pub name: String,
    pub kind: Vec<String>,
    pub src_path: String,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ArtifactProfile {
    pub opt_level: String,
    pub debuginfo: Option<u32>,
    pub test: bool,
}
pub fn parse_cargo_message(line: &str) -> Option<CargoMessage> {
    println!("🔍 {}", "Parsing cargo message requires captain binary".bright_blue());
    delegate_to_captain(vec!["parser", "parse", line])
        .ok()
        .and_then(|_| serde_json::from_str(line).ok())
}
pub fn format_error(msg: &DiagnosticMessage) -> ParsedError {
    println!("🔍 {}", "Formatting error requires captain binary".bright_blue());
    delegate_to_captain(vec!["parser", "format", "error", & msg.message]).ok();
    let code = msg
        .code
        .as_ref()
        .map(|c| c.code.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let (file, line) = if !msg.spans.is_empty() {
        (msg.spans[0].file_name.clone(), msg.spans[0].line_start)
    } else {
        ("unknown".to_string(), 0)
    };
    ParsedError {
        code,
        file,
        line,
        message: msg.message.clone(),
    }
}
pub fn format_warning(msg: &DiagnosticMessage) -> ParsedWarning {
    println!("🔍 {}", "Formatting warning requires captain binary".bright_blue());
    delegate_to_captain(vec!["parser", "format", "warning", & msg.message]).ok();
    let code = msg
        .code
        .as_ref()
        .map(|c| c.code.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let (file, line) = if !msg.spans.is_empty() {
        (msg.spans[0].file_name.clone(), msg.spans[0].line_start)
    } else {
        ("unknown".to_string(), 0)
    };
    ParsedWarning {
        code,
        file,
        line,
        message: msg.message.clone(),
    }
}
pub fn delegate_to_captain(args: Vec<&str>) -> Result<()> {
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
                        "💡 {}", "Parser features require the captain binary:".cyan()
                    );
                    println!("   • Advanced cargo message parsing");
                    println!("   • Error and warning formatting");
                    println!("   • Compiler artifact analysis");
                    println!("   • Build script execution tracking");
                    return Ok(());
                }
            }
        }
    };
    let output = Command::new(&captain_path)
        .args(&args)
        .output()
        .context("Failed to execute captain binary for parsing")?;
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
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CargoMessage {
    pub reason: String,
    #[serde(flatten)]
    pub data: MessageData,
}