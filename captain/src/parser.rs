use serde::{Deserialize, Serialize};
use std::fmt;
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CargoMessage {
    pub reason: String,
    #[serde(flatten)]
    pub data: MessageData,
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
pub fn parse_cargo_message(line: &str) -> Option<CargoMessage> {
    serde_json::from_str(line).ok()
}
pub fn format_error(msg: &DiagnosticMessage) -> ParsedError {
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