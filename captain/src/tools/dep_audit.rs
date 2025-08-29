use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command as ProcessCommand;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub struct DepAuditTool;
#[derive(Debug, Clone, Deserialize, Serialize)]
struct DependencyInfo {
    name: String,
    version: String,
    license: Option<String>,
    security_issues: Vec<String>,
    maintenance_status: String,
    is_direct: bool,
}
#[derive(Debug, Deserialize, Serialize)]
struct AuditResult {
    dependencies: Vec<DependencyInfo>,
    summary: AuditSummary,
}
#[derive(Debug, Deserialize, Serialize)]
struct AuditSummary {
    total_deps: usize,
    direct_deps: usize,
    indirect_deps: usize,
    security_issues: usize,
    license_issues: usize,
    maintenance_issues: usize,
}
impl DepAuditTool {
    pub fn new() -> Self {
        Self
    }
    fn parse_cargo_tree(&self) -> Result<Vec<DependencyInfo>> {
        let output = ProcessCommand::new("cargo")
            .args(&["tree", "--format", "{p} {l}"])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to run cargo tree: {}", e),
            ))?;
        if !output.status.success() {
            return Err(
                ToolError::ExecutionFailed("Cargo tree command failed".to_string()),
            );
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut dependencies = Vec::new();
        for line in stdout.lines() {
            if let Some(dep) = self.parse_dependency_line(line) {
                dependencies.push(dep);
            }
        }
        Ok(dependencies)
    }
    fn parse_dependency_line(&self, line: &str) -> Option<DependencyInfo> {
        let line = line.trim();
        let content = if line.starts_with("├──") {
            &line[3..]
        } else if line.starts_with("└──") {
            &line[3..]
        } else if line.starts_with("│") {
            &line[1..]
        } else {
            line
        }
            .trim();
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }
        let name = parts[0].to_string();
        let version = parts[1].to_string();
        let license = if let Some(start) = content.find('(') {
            if let Some(end) = content.find(')') {
                Some(content[start + 1..end].to_string())
            } else {
                None
            }
        } else {
            None
        };
        let is_direct = line.starts_with("├──") && !line.contains("   ");
        Some(DependencyInfo {
            name,
            version,
            license,
            security_issues: Vec::new(),
            maintenance_status: "unknown".to_string(),
            is_direct,
        })
    }
    fn check_licenses(
        &self,
        deps: &mut [DependencyInfo],
        allowed_licenses: &[String],
    ) -> usize {
        let mut issues = 0;
        for dep in deps.iter_mut() {
            if let Some(ref license) = dep.license {
                let license_ok = allowed_licenses
                    .iter()
                    .any(|allowed| {
                        license.contains(allowed) || allowed.contains(license)
                    });
                if !license_ok {
                    issues += 1;
                }
            } else {
                issues += 1;
            }
        }
        issues
    }
    fn check_security(&self, deps: &mut [DependencyInfo]) -> usize {
        let mut issues = 0;
        for dep in deps.iter_mut() {
            if dep.name == "some-old-crate" && dep.version.starts_with("0.") {
                dep.security_issues
                    .push("Known security vulnerability in old versions".to_string());
                issues += 1;
            }
            if dep.version.contains("yanked") {
                dep.security_issues.push("Version has been yanked".to_string());
                issues += 1;
            }
        }
        issues
    }
    fn check_maintenance(&self, deps: &mut [DependencyInfo]) -> usize {
        let mut issues = 0;
        for dep in deps.iter_mut() {
            if dep.version.contains("alpha") || dep.version.contains("beta") {
                dep.maintenance_status = "pre-release".to_string();
            } else if dep.version.starts_with("0.") {
                dep.maintenance_status = "unstable".to_string();
                issues += 1;
            } else {
                dep.maintenance_status = "active".to_string();
            }
        }
        issues
    }
    fn display_results(
        &self,
        result: &AuditResult,
        format: OutputFormat,
        verbose: bool,
    ) {
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(result).unwrap());
            }
            OutputFormat::Table => {
                println!(
                    "{:<30} {:<12} {:<20} {:<10} {:<8}", "Dependency", "Version",
                    "License", "Security", "Maint"
                );
                println!("{}", "─".repeat(85));
                for dep in &result.dependencies {
                    let security_status = if dep.security_issues.is_empty() {
                        "✅".green().to_string()
                    } else {
                        "❌".red().to_string()
                    };
                    let license_status = if dep.license.is_some() {
                        "✅".green().to_string()
                    } else {
                        "❌".red().to_string()
                    };
                    println!(
                        "{:<30} {:<12} {:<20} {:<10} {:<8}", dep.name, dep.version, dep
                        .license.as_ref().unwrap_or(& "Unknown".to_string()),
                        security_status, dep.maintenance_status
                    );
                }
            }
            OutputFormat::Human => {
                println!("{}", "🔍 Dependency Audit Report".bold().blue());
                println!("{}", "═".repeat(50).blue());
                println!("📦 Total Dependencies: {}", result.summary.total_deps);
                println!("   • Direct: {}", result.summary.direct_deps);
                println!("   • Indirect: {}", result.summary.indirect_deps);
                println!();
                let security_color = if result.summary.security_issues == 0 {
                    result.summary.security_issues.to_string().green()
                } else {
                    result.summary.security_issues.to_string().red()
                };
                let license_color = if result.summary.license_issues == 0 {
                    result.summary.license_issues.to_string().green()
                } else {
                    result.summary.license_issues.to_string().red()
                };
                let maint_color = if result.summary.maintenance_issues == 0 {
                    result.summary.maintenance_issues.to_string().green()
                } else {
                    result.summary.maintenance_issues.to_string().red()
                };
                println!("🔒 Security Issues: {}", security_color);
                println!("📄 License Issues: {}", license_color);
                println!("🔧 Maintenance Issues: {}", maint_color);
                println!();
                if verbose {
                    println!("{}", "Detailed Issues:".bold());
                    println!("{}", "─".repeat(30));
                    for dep in &result.dependencies {
                        if !dep.security_issues.is_empty() || dep.license.is_none()
                            || dep.maintenance_status != "active"
                        {
                            println!("{} ({})", dep.name.bold(), dep.version);
                            if !dep.security_issues.is_empty() {
                                for issue in &dep.security_issues {
                                    println!("   🔒 {}", issue.red());
                                }
                            }
                            if dep.license.is_none() {
                                println!("   📄 {}", "No license information".yellow());
                            }
                            if dep.maintenance_status != "active" {
                                println!(
                                    "   🔧 Status: {}", dep.maintenance_status.yellow()
                                );
                            }
                            println!();
                        }
                    }
                }
            }
        }
    }
}
impl Tool for DepAuditTool {
    fn name(&self) -> &'static str {
        "dep-audit"
    }
    fn description(&self) -> &'static str {
        "Enhanced dependency auditing and security checks"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Audit Rust dependencies for security vulnerabilities, license compatibility, and maintenance status",
            )
            .args(
                &[
                    Arg::new("strict")
                        .long("strict")
                        .help("Fail on any issues found")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("ignore")
                        .long("ignore")
                        .short('i')
                        .help("Ignore specific crates (comma-separated)"),
                    Arg::new("licenses")
                        .long("licenses")
                        .help("Allowed licenses (comma-separated)")
                        .default_value("MIT,Apache-2.0,BSD-3-Clause,ISC"),
                    Arg::new("check-security")
                        .long("check-security")
                        .help("Check for security vulnerabilities")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("check-maintenance")
                        .long("check-maintenance")
                        .help("Check maintenance status")
                        .action(clap::ArgAction::SetTrue),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let strict = matches.get_flag("strict");
        let ignore_list = matches
            .get_one::<String>("ignore")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect::<HashSet<_>>())
            .unwrap_or_default();
        let allowed_licenses: Vec<String> = matches
            .get_one::<String>("licenses")
            .unwrap()
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let check_security = matches.get_flag("check-security")
            || !matches.contains_id("check-security");
        let check_maintenance = matches.get_flag("check-maintenance")
            || !matches.contains_id("check-maintenance");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");
        println!("🔍 {} - Auditing dependencies", "CargoMate DepAudit".bold().blue());
        if !Path::new("Cargo.toml").exists() {
            return Err(
                ToolError::ExecutionFailed(
                    "Not in a Rust project (Cargo.toml not found)".to_string(),
                ),
            );
        }
        let mut dependencies = self.parse_cargo_tree()?;
        dependencies.retain(|dep| !ignore_list.contains(&dep.name));
        let mut security_issues = 0;
        let mut license_issues = 0;
        let mut maintenance_issues = 0;
        if check_security {
            security_issues = self.check_security(&mut dependencies);
        }
        license_issues = self.check_licenses(&mut dependencies, &allowed_licenses);
        if check_maintenance {
            maintenance_issues = self.check_maintenance(&mut dependencies);
        }
        let result = AuditResult {
            dependencies: dependencies.clone(),
            summary: AuditSummary {
                total_deps: dependencies.len(),
                direct_deps: dependencies.iter().filter(|d| d.is_direct).count(),
                indirect_deps: dependencies.iter().filter(|d| !d.is_direct).count(),
                security_issues,
                license_issues,
                maintenance_issues,
            },
        };
        self.display_results(&result, output_format, verbose);
        if strict
            && (security_issues > 0 || license_issues > 0 || maintenance_issues > 0)
        {
            println!();
            println!(
                "{}", "❌ Audit failed due to issues found in strict mode".red().bold()
            );
            std::process::exit(1);
        }
        if security_issues > 0 {
            println!();
            println!(
                "{}", "⚠️  Security issues found - review carefully".yellow().bold()
            );
        }
        Ok(())
    }
}
impl Default for DepAuditTool {
    fn default() -> Self {
        Self::new()
    }
}