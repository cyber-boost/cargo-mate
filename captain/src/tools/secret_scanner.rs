use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub struct SecretScannerTool;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretScanReport {
    files_scanned: usize,
    secrets_found: usize,
    findings: Vec<SecretFinding>,
    false_positives: Vec<FalsePositive>,
    statistics: ScanStatistics,
    recommendations: Vec<String>,
    timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretFinding {
    file_path: String,
    line_number: usize,
    secret_type: String,
    confidence: String,
    context: String,
    secret_value: String,
    masked_secret: String,
    severity: String,
    recommendation: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FalsePositive {
    file_path: String,
    line_number: usize,
    pattern_matched: String,
    reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScanStatistics {
    high_confidence: usize,
    medium_confidence: usize,
    low_confidence: usize,
    false_positives: usize,
    files_with_secrets: usize,
    most_common_type: String,
}
impl SecretScannerTool {
    pub fn new() -> Self {
        Self
    }
    fn get_secret_patterns(&self) -> Vec<SecretPattern> {
        vec![
            SecretPattern { name : "AWS Access Key".to_string(), pattern :
            r"AKIA[0-9A-Z]{16}".to_string(), confidence : "high".to_string(), description
            : "AWS Access Key ID".to_string(), example : "AKIAIOSFODNN7EXAMPLE"
            .to_string(), }, SecretPattern { name : "AWS Secret Key".to_string(), pattern
            : r"[0-9a-zA-Z/+]{40}".to_string(), confidence : "medium".to_string(),
            description : "AWS Secret Access Key".to_string(), example :
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(), }, SecretPattern {
            name : "GitHub Token".to_string(), pattern : r"ghp_[0-9a-zA-Z]{36}"
            .to_string(), confidence : "high".to_string(), description :
            "GitHub Personal Access Token".to_string(), example :
            "ghp_1234567890abcdef1234567890abcdef1234".to_string(), }, SecretPattern {
            name : "GitHub OAuth".to_string(), pattern : r"gho_[0-9a-zA-Z]{36}"
            .to_string(), confidence : "high".to_string(), description :
            "GitHub OAuth Access Token".to_string(), example :
            "gho_1234567890abcdef1234567890abcdef1234".to_string(), }, SecretPattern {
            name : "GitLab Token".to_string(), pattern : r"glpat-[0-9a-zA-Z\-_]{20}"
            .to_string(), confidence : "high".to_string(), description :
            "GitLab Personal Access Token".to_string(), example :
            "glpat-1234567890abcdefghij".to_string(), }, SecretPattern { name :
            "Slack Token".to_string(), pattern : r"xox[baprs]-[0-9a-zA-Z]{10,48}"
            .to_string(), confidence : "high".to_string(), description :
            "Slack API Token".to_string(), example :
            "xoxb-1234567890-1234567890-abcdefghijklmnopqrstuvwx".to_string(), },
            SecretPattern { name : "Discord Token".to_string(), pattern :
            r"[MN][A-Za-z\d]{23}\.[\w-]{6}\.[\w-]{27}".to_string(), confidence : "high"
            .to_string(), description : "Discord Bot Token".to_string(), example :
            "MTAwMDAwMDAwMDAwMDAwMDAw.GQ0wHm.example".to_string(), }, SecretPattern {
            name : "Stripe Key".to_string(), pattern : r"[rs]k_live_[0-9a-zA-Z]{24}"
            .to_string(), confidence : "high".to_string(), description : "Stripe API Key"
            .to_string(), example : "sk_live_1234567890abcdefghijklmnopqrstuvwxyz"
            .to_string(), }, SecretPattern { name : "Database URL".to_string(), pattern :
            r"(?i)postgres://[^:\s]+:[^@\s]+@".to_string(), confidence : "high"
            .to_string(), description : "PostgreSQL connection string".to_string(),
            example : "postgres://username:password@host:port/database".to_string(), },
            SecretPattern { name : "Generic API Key".to_string(), pattern :
            r#"(?i)(api[_-]?key|apikey|secret|token|auth[_-]?token)\s*[:=]\s*['"]?.*['"]?"#
            .to_string(), confidence : "medium".to_string(), description :
            "Generic API key or token".to_string(), example :
            "api_key=1234567890abcdef1234567890abcdef".to_string(), }, SecretPattern {
            name : "Private Key".to_string(), pattern :
            r"-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----".to_string(), confidence :
            "high".to_string(), description : "RSA Private Key".to_string(), example :
            "-----BEGIN PRIVATE KEY-----".to_string(), }, SecretPattern { name :
            "SSH Private Key".to_string(), pattern :
            r"-----BEGIN\s+OPENSSH\s+PRIVATE\s+KEY-----".to_string(), confidence : "high"
            .to_string(), description : "SSH Private Key".to_string(), example :
            "-----BEGIN OPENSSH PRIVATE KEY-----".to_string(), }, SecretPattern { name :
            "JWT Token".to_string(), pattern :
            r#"eyJ[A-Za-z0-9_.-]*\.[A-Za-z0-9_.-]*\.[A-Za-z0-9_.-]*"#.to_string(),
            confidence : "medium".to_string(), description : "JSON Web Token"
            .to_string(), example :
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"
            .to_string(), }, SecretPattern { name : "Password in Config".to_string(),
            pattern : r#"(?i)(password|passwd|pwd)\s*[:=]\s*['"]?.*['"]?"#.to_string(),
            confidence : "low".to_string(), description :
            "Potential password in configuration".to_string(), example :
            "password=secret123".to_string(), }, SecretPattern { name : "Bearer Token"
            .to_string(), pattern : r#"(?i)bearer\s+[^'"\s]+"#.to_string(), confidence :
            "medium".to_string(), description : "Bearer token in Authorization header"
            .to_string(), example : "Authorization: Bearer abcdef1234567890".to_string(),
            },
        ]
    }
    fn scan_file_for_secrets(&self, file_path: &str) -> Result<Vec<SecretFinding>> {
        let content = fs::read_to_string(file_path)?;
        let mut findings = Vec::new();
        let file_name = Path::new(file_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        if file_name.ends_with(".min.js") || file_name.ends_with(".min.css")
            || file_name.contains("jquery") || file_name.contains("bootstrap")
        {
            return Ok(vec![]);
        }
        let patterns = self.get_secret_patterns();
        for (line_number, line) in content.lines().enumerate() {
            for pattern in &patterns {
                if let Ok(regex) = Regex::new(&pattern.pattern) {
                    if let Some(captures) = regex.captures(line) {
                        if let Some(secret_match) = captures
                            .get(1)
                            .or_else(|| captures.get(0))
                        {
                            let secret_value = secret_match.as_str().to_string();
                            if self.is_false_positive(&secret_value, pattern, line) {
                                continue;
                            }
                            let masked_secret = self.mask_secret(&secret_value);
                            let context = self.extract_context(&content, line_number, 2);
                            let recommendation = self
                                .generate_recommendation(pattern, file_path);
                            findings
                                .push(SecretFinding {
                                    file_path: file_path.to_string(),
                                    line_number: line_number + 1,
                                    secret_type: pattern.name.clone(),
                                    confidence: pattern.confidence.clone(),
                                    context,
                                    secret_value: secret_value.clone(),
                                    masked_secret,
                                    severity: self
                                        .calculate_severity(&pattern.confidence, &secret_value),
                                    recommendation,
                                });
                        }
                    }
                }
            }
        }
        Ok(findings)
    }
    fn is_false_positive(
        &self,
        secret: &str,
        pattern: &SecretPattern,
        line: &str,
    ) -> bool {
        let false_positive_patterns = vec![
            "example", "test", "demo", "sample", "placeholder", "your", "fake", "dummy",
            "mock", "xxx", "1234567890",
        ];
        let lower_secret = secret.to_lowercase();
        let lower_line = line.to_lowercase();
        if lower_line.contains("//")
            && lower_line.find("//").unwrap()
                < lower_line.find(&lower_secret).unwrap_or(0)
        {
            return true;
        }
        for pattern in false_positive_patterns {
            if lower_secret.contains(pattern) {
                return true;
            }
        }
        if secret.len() < 10 && secret.chars().all(|c| c.is_alphabetic()) {
            return true;
        }
        false
    }
    fn mask_secret(&self, secret: &str) -> String {
        if secret.len() <= 4 {
            return "*".repeat(secret.len());
        }
        let visible_chars = 2;
        let masked_chars = secret.len() - (visible_chars * 2);
        format!(
            "{}{}{}", & secret[0..visible_chars], "*".repeat(masked_chars), &
            secret[secret.len() - visible_chars..]
        )
    }
    fn extract_context(
        &self,
        content: &str,
        line_number: usize,
        context_lines: usize,
    ) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let start = line_number.saturating_sub(context_lines);
        let end = std::cmp::min(line_number + context_lines + 1, lines.len());
        lines[start..end].join("\n")
    }
    fn calculate_severity(&self, confidence: &str, secret: &str) -> String {
        match confidence {
            "high" => {
                if secret.len() > 20 {
                    "critical".to_string()
                } else {
                    "high".to_string()
                }
            }
            "medium" => {
                if secret.len() > 15 { "high".to_string() } else { "medium".to_string() }
            }
            "low" => {
                if secret.len() > 20 { "medium".to_string() } else { "low".to_string() }
            }
            _ => "medium".to_string(),
        }
    }
    fn generate_recommendation(
        &self,
        pattern: &SecretPattern,
        file_path: &str,
    ) -> String {
        let base_recommendation = match pattern.name.as_str() {
            "AWS Access Key" | "AWS Secret Key" => {
                "Move to AWS IAM roles or use environment variables with proper access control"
            }
            "GitHub Token" | "GitHub OAuth" => {
                "Use GitHub Actions secrets or environment variables"
            }
            "GitLab Token" => "Use GitLab CI/CD variables or environment variables",
            "Slack Token" => "Use Slack app configuration or environment variables",
            "Discord Token" => "Use environment variables or secure configuration files",
            "Stripe Key" => "Use environment variables and never commit live keys",
            "Database URL" => {
                "Use environment variables or connection configuration files"
            }
            "Generic API Key" => {
                "Use environment variables or secure configuration management"
            }
            "Private Key" | "SSH Private Key" => "Store in secure key management system",
            "JWT Token" => "Use environment variables and proper token rotation",
            "Password in Config" => {
                "Use secure password hashing and environment variables"
            }
            "Bearer Token" => "Use environment variables and secure token storage",
            _ => "Use environment variables or secure configuration management",
        };
        format!(
            "{} (found in {})", base_recommendation, file_path.split('/').last()
            .unwrap_or(file_path)
        )
    }
    fn find_files_to_scan(&self, directory: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();
        self.find_files_recursive(directory, &mut files)?;
        Ok(files)
    }
    fn find_files_recursive(&self, dir: &str, files: &mut Vec<String>) -> Result<()> {
        let path = Path::new(dir);
        if !path.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if !matches!(
                    dir_name.as_ref(), "target" | ".git" | "node_modules" | ".cargo" |
                    ".vscode" | ".idea" | ".DS_Store" | "dist" | "build" | "out"
                ) {
                    self.find_files_recursive(&path.to_string_lossy(), files)?;
                }
            } else if let Some(ext) = path.extension() {
                match ext.to_string_lossy().as_ref() {
                    "rs" | "toml" | "json" | "yaml" | "yml" | "env" | "config" | "ini"
                    | "cfg" | "properties" | "sh" | "bash" | "py" | "js" | "ts" | "php"
                    | "rb" | "go" | "java" | "kt" | "scala" => {
                        files.push(path.to_string_lossy().to_string());
                    }
                    _ => {}
                }
            } else if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if matches!(
                    file_name_str.as_ref(), ".env" | ".env.local" | ".env.production" |
                    "secrets" | "config" | "settings" | "credentials"
                ) {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
        Ok(())
    }
    fn calculate_statistics(&self, findings: &[SecretFinding]) -> ScanStatistics {
        let mut high_confidence = 0;
        let mut medium_confidence = 0;
        let mut low_confidence = 0;
        let mut files_with_secrets = std::collections::HashSet::new();
        let mut type_counts = std::collections::HashMap::new();
        for finding in findings {
            files_with_secrets.insert(&finding.file_path);
            match finding.confidence.as_str() {
                "high" => high_confidence += 1,
                "medium" => medium_confidence += 1,
                "low" => low_confidence += 1,
                _ => {}
            }
            *type_counts.entry(&finding.secret_type).or_insert(0) += 1;
        }
        let most_common_type = type_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(ty, _)| ty.clone())
            .unwrap_or_else(|| "None".to_string());
        ScanStatistics {
            high_confidence,
            medium_confidence,
            low_confidence,
            false_positives: 0,
            files_with_secrets: files_with_secrets.len(),
            most_common_type,
        }
    }
    fn generate_recommendations(&self, findings: &[SecretFinding]) -> Vec<String> {
        let mut recommendations = Vec::new();
        if !findings.is_empty() {
            recommendations
                .push(
                    "Use environment variables instead of hardcoding secrets".to_string(),
                );
            recommendations
                .push(
                    "Store secrets in secure configuration management systems"
                        .to_string(),
                );
            recommendations
                .push(
                    "Use .env files for local development (add to .gitignore)"
                        .to_string(),
                );
            recommendations
                .push(
                    "Implement pre-commit hooks to prevent secret commits".to_string(),
                );
            recommendations.push("Rotate exposed secrets immediately".to_string());
            recommendations.push("Use secret scanning in CI/CD pipelines".to_string());
        }
        let has_high_severity = findings.iter().any(|f| f.severity == "critical");
        if has_high_severity {
            recommendations
                .insert(
                    0,
                    "🚨 CRITICAL: High-severity secrets found - rotate immediately!"
                        .to_string(),
                );
        }
        recommendations
    }
    fn display_report(
        &self,
        report: &SecretScanReport,
        output_format: OutputFormat,
        verbose: bool,
    ) {
        match output_format {
            OutputFormat::Human => {
                println!(
                    "\n🔍 {} - Secret Scanner Report", "CargoMate SecretScanner".bold()
                    .blue()
                );
                println!("{}", "═".repeat(60).blue());
                println!("\n📊 Summary:");
                println!("  • Files Scanned: {}", report.files_scanned);
                println!("  • Secrets Found: {}", report.secrets_found);
                println!(
                    "  • Files with Secrets: {}", report.statistics.files_with_secrets
                );
                println!("  • High Confidence: {}", report.statistics.high_confidence);
                println!(
                    "  • Medium Confidence: {}", report.statistics.medium_confidence
                );
                println!("  • Low Confidence: {}", report.statistics.low_confidence);
                println!(
                    "  • Most Common Type: {}", report.statistics.most_common_type
                );
                if !report.findings.is_empty() {
                    println!("\n🚨 Secrets Found:");
                    for finding in &report.findings {
                        let severity_icon = match finding.severity.as_str() {
                            "critical" => "🚨",
                            "high" => "❌",
                            "medium" => "⚠️",
                            "low" => "ℹ️",
                            _ => "•",
                        };
                        let confidence_icon = match finding.confidence.as_str() {
                            "high" => "🎯",
                            "medium" => "🔍",
                            "low" => "❓",
                            _ => "•",
                        };
                        println!(
                            "  {} {} {} - {} (Line {})", severity_icon, confidence_icon,
                            finding.secret_type.red(), finding.file_path.split('/')
                            .last().unwrap_or(& finding.file_path), finding.line_number
                        );
                        if verbose {
                            println!(
                                "    🔒 Secret: {}", finding.masked_secret.dimmed()
                            );
                            println!("    📝 Context:");
                            for line in finding.context.lines() {
                                if line.contains(&finding.secret_value) {
                                    println!("    > {}", line.red());
                                } else {
                                    println!("    > {}", line.dimmed());
                                }
                            }
                            println!("    💡 {}", finding.recommendation.cyan());
                        }
                    }
                }
                if !report.recommendations.is_empty() {
                    println!("\n💡 Recommendations:");
                    for rec in &report.recommendations {
                        println!("  • {}", rec.cyan());
                    }
                }
                println!("\n✅ Scan complete!");
                if report.secrets_found == 0 {
                    println!("   No secrets found - good job!");
                } else {
                    println!(
                        "   Found {} potential secret(s) to review", report.secrets_found
                    );
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!(
                    "{:<35} {:<12} {:<15} {:<10} {:<20}", "File", "Line", "Type",
                    "Severity", "Masked Secret"
                );
                println!("{}", "─".repeat(100));
                for finding in &report.findings {
                    let file_name = finding
                        .file_path
                        .split('/')
                        .last()
                        .unwrap_or(&finding.file_path);
                    println!(
                        "{:<35} {:<12} {:<15} {:<10} {:<20}", file_name.chars().take(34)
                        .collect::< String > (), finding.line_number.to_string(), finding
                        .secret_type.chars().take(14).collect::< String > (), finding
                        .severity, finding.masked_secret.chars().take(19).collect::<
                        String > ()
                    );
                }
            }
        }
    }
}
#[derive(Debug, Clone)]
struct SecretPattern {
    name: String,
    pattern: String,
    confidence: String,
    description: String,
    example: String,
}
impl Tool for SecretScannerTool {
    fn name(&self) -> &'static str {
        "secret-scanner"
    }
    fn description(&self) -> &'static str {
        "Scan for hardcoded secrets and API keys"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Scan your codebase for hardcoded secrets, API keys, tokens, and other \
                        sensitive information that should not be committed to version control.

EXAMPLES:
    cm tool secret-scanner --directory src/
    cm tool secret-scanner --workspace --exclude-vendor
    cm tool secret-scanner --format json --output secrets.json",
            )
            .args(
                &[
                    Arg::new("directory")
                        .long("directory")
                        .short('d')
                        .help("Directory to scan (default: current)")
                        .default_value("."),
                    Arg::new("workspace")
                        .long("workspace")
                        .help("Scan entire workspace")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("exclude-vendor")
                        .long("exclude-vendor")
                        .help("Exclude vendor directories")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("include-tests")
                        .long("include-tests")
                        .help("Include test files in scan")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("confidence")
                        .long("confidence")
                        .short('c')
                        .help("Minimum confidence level")
                        .default_value("low")
                        .value_parser(["low", "medium", "high"]),
                    Arg::new("format")
                        .long("format")
                        .short('f')
                        .help("Output format for secrets")
                        .default_value("masked")
                        .value_parser(["masked", "full", "none"]),
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output file for results"),
                    Arg::new("ci-mode")
                        .long("ci-mode")
                        .help("CI-friendly output with exit codes")
                        .action(clap::ArgAction::SetTrue),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let directory = matches.get_one::<String>("directory").unwrap();
        let workspace = matches.get_flag("workspace");
        let exclude_vendor = matches.get_flag("exclude-vendor");
        let include_tests = matches.get_flag("include-tests");
        let min_confidence = matches.get_one::<String>("confidence").unwrap();
        let format = matches.get_one::<String>("format").unwrap();
        let output_file = matches.get_one::<String>("output");
        let ci_mode = matches.get_flag("ci-mode");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");
        println!(
            "🔍 {} - Scanning for Secrets", "CargoMate SecretScanner".bold().blue()
        );
        let scan_directory = if workspace { ".".to_string() } else { directory.clone() };
        let files_to_scan = self.find_files_to_scan(&scan_directory)?;
        if files_to_scan.is_empty() {
            println!("{}", "No files found to scan".yellow());
            return Ok(());
        }
        let mut all_findings = Vec::new();
        for file_path in &files_to_scan {
            match self.scan_file_for_secrets(file_path) {
                Ok(findings) => {
                    for finding in findings {
                        let include_finding = match min_confidence.as_str() {
                            "high" => finding.confidence == "high",
                            "medium" => {
                                finding.confidence == "high"
                                    || finding.confidence == "medium"
                            }
                            "low" => true,
                            _ => true,
                        };
                        if include_finding {
                            all_findings.push(finding);
                        }
                    }
                }
                Err(e) => {
                    if verbose {
                        println!("⚠️  Failed to scan {}: {}", file_path, e);
                    }
                }
            }
        }
        let final_findings = all_findings
            .into_iter()
            .filter(|finding| {
                match format.as_str() {
                    "none" => false,
                    "masked" => true,
                    "full" => true,
                    _ => true,
                }
            })
            .collect::<Vec<_>>();
        let statistics = self.calculate_statistics(&final_findings);
        let recommendations = self.generate_recommendations(&final_findings);
        let report = SecretScanReport {
            files_scanned: files_to_scan.len(),
            secrets_found: final_findings.len(),
            findings: final_findings,
            false_positives: Vec::new(),
            statistics,
            recommendations,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        if let Some(output_path) = output_file {
            let json = serde_json::to_string_pretty(&report)?;
            fs::write(output_path, json)?;
            println!("📄 Report saved to {}", output_path);
        }
        self.display_report(&report, output_format, verbose);
        if ci_mode && !report.findings.is_empty() {
            let critical_count = report
                .findings
                .iter()
                .filter(|f| f.severity == "critical")
                .count();
            let high_count = report
                .findings
                .iter()
                .filter(|f| f.severity == "high")
                .count();
            println!("::set-output name=secrets-found::{}", report.secrets_found);
            println!("::set-output name=secrets-critical::{}", critical_count);
            println!("::set-output name=secrets-high::{}", high_count);
            if critical_count > 0 {
                println!(
                    "::error title=Critical Secrets Found::Found {} critical secrets that must be addressed",
                    critical_count
                );
            }
            if critical_count > 0 || high_count > 0 {
                std::process::exit(1);
            }
        }
        Ok(())
    }
}
impl Default for SecretScannerTool {
    fn default() -> Self {
        Self::new()
    }
}