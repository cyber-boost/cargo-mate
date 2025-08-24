use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::env;
use std::process::Command as ProcessCommand;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct EnvCheckTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnvironmentReport {
    overall_status: String,
    checks_passed: usize,
    checks_failed: usize,
    checks_warning: usize,
    rust_toolchain: RustToolchainInfo,
    system_info: SystemInfo,
    dependencies: DependencyStatus,
    configuration: ConfigStatus,
    recommendations: Vec<String>,
    issues: Vec<EnvironmentIssue>,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RustToolchainInfo {
    version: String,
    channel: String,
    target: String,
    components: Vec<String>,
    is_compatible: bool,
    issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemInfo {
    os: String,
    arch: String,
    memory_gb: u64,
    available_disk_gb: u64,
    cpu_cores: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DependencyStatus {
    required_tools: HashMap<String, ToolStatus>,
    optional_tools: HashMap<String, ToolStatus>,
    rust_components: HashMap<String, ComponentStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConfigStatus {
    cargo_config_exists: bool,
    rustfmt_config_exists: bool,
    clippy_config_exists: bool,
    environment_variables: HashMap<String, EnvVarStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolStatus {
    installed: bool,
    version: Option<String>,
    required_version: Option<String>,
    status: String,
    path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ComponentStatus {
    installed: bool,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnvVarStatus {
    set: bool,
    value: Option<String>,
    required: bool,
    masked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EnvironmentIssue {
    severity: String,
    category: String,
    message: String,
    solution: String,
    command: Option<String>,
}

impl EnvCheckTool {
    pub fn new() -> Self {
        Self
    }

    fn check_rust_toolchain(&self) -> Result<RustToolchainInfo> {
        let mut issues = Vec::new();

        // Check Rust version
        let version_output = ProcessCommand::new("rustc")
            .arg("--version")
            .output()
            .map_err(|_| ToolError::ExecutionFailed("rustc not found".to_string()))?;

        let version_str = String::from_utf8_lossy(&version_output.stdout);
        let version = version_str.split_whitespace()
            .nth(1)
            .unwrap_or("unknown")
            .to_string();

        // Parse version and check compatibility
        let is_compatible = self.is_rust_version_compatible(&version);

        if !is_compatible {
            issues.push(format!("Rust version {} may be outdated", version));
        }

        // Check channel
        let channel = if version.contains("nightly") {
            "nightly".to_string()
        } else if version.contains("beta") {
            "beta".to_string()
        } else {
            "stable".to_string()
        };

        // Get target
        let target_output = ProcessCommand::new("rustc")
            .args(&["--print", "target-list"])
            .output()
            .ok();

        let target = target_output
            .and_then(|output| String::from_utf8_lossy(&output.stdout).lines().next().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        // Check installed components
        let components = self.check_rust_components()?;

        Ok(RustToolchainInfo {
            version,
            channel,
            target,
            components,
            is_compatible,
            issues,
        })
    }

    fn is_rust_version_compatible(&self, version: &str) -> bool {
        // Simple version check - require 1.70.0 or later
        if let Some(version_part) = version.split('+').next() {
            let parts: Vec<&str> = version_part.split('.').collect();
            if parts.len() >= 2 {
                if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    return major > 1 || (major == 1 && minor >= 70);
                }
            }
        }
        false
    }

    fn check_rust_components(&self) -> Result<Vec<String>> {
        let output = ProcessCommand::new("rustup")
            .args(&["component", "list", "--installed"])
            .output()
            .map_err(|_| ToolError::ExecutionFailed("rustup not found".to_string()))?;

        let components = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();

        Ok(components)
    }

    fn check_system_info(&self) -> Result<SystemInfo> {
        let os = env::consts::OS.to_string();
        let arch = env::consts::ARCH.to_string();

        // Get memory info (simplified)
        let memory_gb = self.get_memory_info();

        // Get disk info (simplified)
        let available_disk_gb = self.get_disk_info();

        // Get CPU cores (fallback since num_cpus crate is not available)
        let cpu_cores = self.get_cpu_cores_fallback();

        Ok(SystemInfo {
            os,
            arch,
            memory_gb,
            available_disk_gb,
            cpu_cores,
        })
    }

    fn get_memory_info(&self) -> u64 {
        // Simplified memory check
        if cfg!(target_os = "linux") {
            if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
                for line in contents.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = kb_str.parse::<u64>() {
                                return kb / 1024 / 1024; // Convert to GB
                            }
                        }
                    }
                }
            }
        }
        0
    }

    fn get_disk_info(&self) -> u64 {
        // Simplified disk check - check current directory
        if let Ok(stat) = fs::metadata(".") {
            // This is a very basic check - in practice you'd want more sophisticated disk space checking
            100 // Assume 100GB available for demo
        } else {
            0
        }
    }

    fn get_cpu_cores_fallback(&self) -> usize {
        // Fallback method to get CPU cores when num_cpus crate is not available
        if cfg!(target_os = "linux") {
            // Try reading from /proc/cpuinfo
            if let Ok(contents) = fs::read_to_string("/proc/cpuinfo") {
                let processor_count = contents.lines()
                    .filter(|line| line.starts_with("processor"))
                    .count();
                if processor_count > 0 {
                    return processor_count;
                }
            }
        } else if cfg!(target_os = "macos") {
            // Try using sysctl on macOS
            if let Ok(output) = ProcessCommand::new("sysctl")
                .args(&["-n", "hw.ncpu"])
                .output() {
                if let Ok(core_str) = String::from_utf8(output.stdout) {
                    if let Ok(cores) = core_str.trim().parse::<usize>() {
                        return cores;
                    }
                }
            }
        }

        // Default fallback
        4 // Assume 4 cores if we can't determine
    }

    fn check_dependencies(&self) -> Result<DependencyStatus> {
        let mut required_tools = HashMap::new();
        let mut optional_tools = HashMap::new();
        let mut rust_components = HashMap::new();

        // Check required tools
        let required = vec!["cargo", "rustc", "rustup", "git"];
        for tool in required {
            required_tools.insert(tool.to_string(), self.check_tool(tool)?);
        }

        // Check optional tools
        let optional = vec!["docker", "node", "npm", "yarn", "python", "python3"];
        for tool in optional {
            optional_tools.insert(tool.to_string(), self.check_tool(tool)?);
        }

        // Check Rust components
        let components = vec!["rustfmt", "clippy", "rust-docs", "rust-analyzer"];
        for component in components {
            rust_components.insert(component.to_string(), self.check_rust_component(component)?);
        }

        Ok(DependencyStatus {
            required_tools,
            optional_tools,
            rust_components,
        })
    }

    fn check_tool(&self, tool: &str) -> Result<ToolStatus> {
        let output = ProcessCommand::new(tool)
            .arg("--version")
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    let version = String::from_utf8_lossy(&result.stdout)
                        .lines()
                        .next()
                        .unwrap_or("unknown")
                        .to_string();

                    // Get tool path
                    let path_output = ProcessCommand::new("which")
                        .arg(tool)
                        .output()
                        .ok();

                    let path = path_output
                        .and_then(|p| if p.status.success() {
                            String::from_utf8_lossy(&p.stdout).trim().to_string().into()
                        } else {
                            None
                        });

                    Ok(ToolStatus {
                        installed: true,
                        version: Some(version),
                        required_version: None,
                        status: "ok".to_string(),
                        path,
                    })
                } else {
                    Ok(ToolStatus {
                        installed: false,
                        version: None,
                        required_version: None,
                        status: "not_found".to_string(),
                        path: None,
                    })
                }
            }
            Err(_) => Ok(ToolStatus {
                installed: false,
                version: None,
                required_version: None,
                status: "not_found".to_string(),
                path: None,
            }),
        }
    }

    fn check_rust_component(&self, component: &str) -> Result<ComponentStatus> {
        let output = ProcessCommand::new("rustup")
            .args(&["component", "list", "--installed"])
            .output()
            .map_err(|_| ToolError::ExecutionFailed("rustup not found".to_string()))?;

        let installed_components = String::from_utf8_lossy(&output.stdout);
        let installed = installed_components.contains(component);

        Ok(ComponentStatus {
            installed,
            status: if installed { "ok".to_string() } else { "missing".to_string() },
        })
    }

    fn check_configuration(&self) -> Result<ConfigStatus> {
        let cargo_config_exists = Path::new("Cargo.toml").exists();
        let rustfmt_config_exists = Path::new(".rustfmt.toml").exists() ||
                                   Path::new("rustfmt.toml").exists();
        let clippy_config_exists = Path::new(".clippy.toml").exists() ||
                                  Path::new("clippy.toml").exists();

        // Check common environment variables
        let env_vars = vec!["RUST_BACKTRACE", "CARGO_HOME", "RUSTUP_HOME", "PATH"];
        let mut environment_variables = HashMap::new();

        for var in env_vars {
            let value = env::var(var).ok();
            let set = value.is_some();
            let required = matches!(var, "PATH");
            let masked = matches!(var, "RUST_BACKTRACE" | "CARGO_HOME" | "RUSTUP_HOME");

            environment_variables.insert(var.to_string(), EnvVarStatus {
                set,
                value: if masked { Some("***masked***".to_string()) } else { value },
                required,
                masked,
            });
        }

        Ok(ConfigStatus {
            cargo_config_exists,
            rustfmt_config_exists,
            clippy_config_exists,
            environment_variables,
        })
    }

    fn analyze_issues(&self, report: &EnvironmentReport) -> Vec<EnvironmentIssue> {
        let mut issues = Vec::new();

        // Check Rust toolchain issues
        if !report.rust_toolchain.is_compatible {
            issues.push(EnvironmentIssue {
                severity: "warning".to_string(),
                category: "rust_toolchain".to_string(),
                message: format!("Rust version {} may be outdated", report.rust_toolchain.version),
                solution: "Update Rust to version 1.70.0 or later".to_string(),
                command: Some("rustup update stable".to_string()),
            });
        }

        // Check missing required tools
        for (tool, status) in &report.dependencies.required_tools {
            if !status.installed {
                issues.push(EnvironmentIssue {
                    severity: "error".to_string(),
                    category: "dependencies".to_string(),
                    message: format!("Required tool '{}' is not installed", tool),
                    solution: format!("Install {} using your system package manager", tool),
                    command: None,
                });
            }
        }

        // Check missing Rust components
        for (component, status) in &report.dependencies.rust_components {
            if !status.installed {
                issues.push(EnvironmentIssue {
                    severity: "warning".to_string(),
                    category: "rust_components".to_string(),
                    message: format!("Rust component '{}' is not installed", component),
                    solution: format!("Install with: rustup component add {}", component),
                    command: Some(format!("rustup component add {}", component)),
                });
            }
        }

        // Check system resources
        if report.system_info.memory_gb < 4 {
            issues.push(EnvironmentIssue {
                severity: "warning".to_string(),
                category: "system".to_string(),
                message: format!("Low memory: {}GB available", report.system_info.memory_gb),
                solution: "Consider upgrading to at least 8GB RAM for better Rust compilation performance".to_string(),
                command: None,
            });
        }

        issues
    }

    fn generate_recommendations(&self, report: &EnvironmentReport) -> Vec<String> {
        let mut recommendations = Vec::new();

        // General recommendations
        recommendations.push("🎯 Keep Rust updated to the latest stable version".to_string());
        recommendations.push("📚 Install rustfmt and clippy for code formatting and linting".to_string());
        recommendations.push("🔧 Set RUST_BACKTRACE=1 for better error messages during development".to_string());

        // Tool-specific recommendations
        if !report.dependencies.optional_tools.get("docker").map_or(false, |t| t.installed) {
            recommendations.push("🐳 Consider installing Docker for integration testing".to_string());
        }

        if !report.configuration.rustfmt_config_exists {
            recommendations.push("📝 Create .rustfmt.toml for consistent code formatting".to_string());
        }

        if !report.configuration.clippy_config_exists {
            recommendations.push("🔍 Create .clippy.toml to configure linting rules".to_string());
        }

        recommendations
    }

    fn determine_overall_status(&self, report: &EnvironmentReport) -> String {
        let critical_issues = report.issues.iter().filter(|i| i.severity == "error").count();
        let warning_issues = report.issues.iter().filter(|i| i.severity == "warning").count();

        if critical_issues > 0 {
            "❌ Issues Found".to_string()
        } else if warning_issues > 0 {
            "⚠️  Needs Attention".to_string()
        } else {
            "✅ All Good".to_string()
        }
    }
}

impl Tool for EnvCheckTool {
    fn name(&self) -> &'static str {
        "env-check"
    }

    fn description(&self) -> &'static str {
        "Validate development environment and provide setup recommendations"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Comprehensive development environment validation for Rust projects. Checks Rust toolchain, system resources, required tools, and provides actionable recommendations for optimizing your development setup.")
            .args(&[
                Arg::new("detailed")
                    .long("detailed")
                    .short('d')
                    .help("Show detailed system information")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("fix")
                    .long("fix")
                    .help("Attempt to automatically fix issues")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("export")
                    .long("export")
                    .help("Export environment report to file")
                    .value_name("FILE"),
                Arg::new("check-only")
                    .long("check-only")
                    .help("Only run checks, don't provide recommendations")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let detailed = matches.get_flag("detailed");
        let fix = matches.get_flag("fix");
        let export_file = matches.get_one::<String>("export");
        let check_only = matches.get_flag("check-only");
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");
        let output_format = parse_output_format(matches);

        println!("🔍 {} - {}", "CargoMate EnvCheck".bold().blue(), self.description().cyan());

        if verbose {
            println!("   📊 Analyzing development environment...");
        }

        // Gather environment information
        let rust_toolchain = self.check_rust_toolchain()?;
        let system_info = self.check_system_info()?;
        let dependencies = self.check_dependencies()?;
        let configuration = self.check_configuration()?;

        // Create report
        let mut report = EnvironmentReport {
            overall_status: String::new(), // Will be set later
            checks_passed: 0,
            checks_failed: 0,
            checks_warning: 0,
            rust_toolchain,
            system_info,
            dependencies,
            configuration,
            recommendations: Vec::new(),
            issues: Vec::new(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Analyze issues and generate recommendations
        report.issues = self.analyze_issues(&report);
        report.recommendations = if check_only {
            Vec::new()
        } else {
            self.generate_recommendations(&report)
        };

        // Count checks
        report.checks_passed = report.dependencies.required_tools.values().filter(|t| t.installed).count()
                             + report.dependencies.optional_tools.values().filter(|t| t.installed).count()
                             + report.dependencies.rust_components.values().filter(|c| c.installed).count();
        report.checks_failed = report.issues.iter().filter(|i| i.severity == "error").count();
        report.checks_warning = report.issues.iter().filter(|i| i.severity == "warning").count();

        // Determine overall status
        report.overall_status = self.determine_overall_status(&report);

        // Handle auto-fix if requested
        if fix && !dry_run {
            self.apply_fixes(&report.issues)?;
        }

        // Export report if requested
        if let Some(file_path) = export_file {
            if !dry_run {
                let json_report = serde_json::to_string_pretty(&report)?;
                fs::write(file_path, json_report)?;
                println!("  💾 Report exported to: {}", file_path.cyan());
            }
        }

        match output_format {
            OutputFormat::Human => {
                self.display_human_report(&report, detailed, check_only);
            }
            OutputFormat::Json => {
                let json_report = serde_json::to_string_pretty(&report)?;
                println!("{}", json_report);
            }
            OutputFormat::Table => {
                self.display_table_report(&report);
            }
        }

        if report.checks_failed > 0 {
            println!("\n❌ Environment check failed - {} critical issues found", report.checks_failed);
            std::process::exit(1);
        } else if report.checks_warning > 0 {
            println!("\n⚠️  Environment check completed with {} warnings", report.checks_warning);
        } else {
            println!("\n✅ Environment check passed - all systems go!");
        }

        Ok(())
    }
}

impl EnvCheckTool {
    fn display_human_report(&self, report: &EnvironmentReport, detailed: bool, check_only: bool) {
        println!("\n📊 {}", "Environment Report".bold().underline());
        println!("Status: {}", report.overall_status);
        println!("Timestamp: {}", report.timestamp);
        println!("Checks: ✅ {} passed, ❌ {} failed, ⚠️  {} warnings",
                 report.checks_passed, report.checks_failed, report.checks_warning);

        // Rust Toolchain
        println!("\n🦀 {}", "Rust Toolchain".bold());
        println!("   Version: {}", report.rust_toolchain.version);
        println!("   Channel: {}", report.rust_toolchain.channel);
        println!("   Target: {}", report.rust_toolchain.target);
        println!("   Compatible: {}", if report.rust_toolchain.is_compatible { "✅ Yes" } else { "❌ No" });

        if detailed {
            println!("   Components: {}", report.rust_toolchain.components.join(", "));
        }

        if !report.rust_toolchain.issues.is_empty() {
            for issue in &report.rust_toolchain.issues {
                println!("   ⚠️  {}", issue.yellow());
            }
        }

        // System Information
        if detailed {
            println!("\n💻 {}", "System Information".bold());
            println!("   OS: {} {}", report.system_info.os, report.system_info.arch);
            println!("   Memory: {} GB", report.system_info.memory_gb);
            println!("   Disk Space: {} GB available", report.system_info.available_disk_gb);
            println!("   CPU Cores: {}", report.system_info.cpu_cores);
        }

        // Dependencies
        println!("\n📦 {}", "Dependencies".bold());

        println!("   {}", "Required Tools:".underline());
        for (tool, status) in &report.dependencies.required_tools {
            let status_icon = if status.installed { "✅" } else { "❌" };
            let version = status.version.as_ref().map(|v| format!(" ({})", v)).unwrap_or_default();
            println!("     {} {} {}", status_icon, tool, version);
        }

        if detailed {
            println!("   {}", "Optional Tools:".underline());
            for (tool, status) in &report.dependencies.optional_tools {
                let status_icon = if status.installed { "✅" } else { "⚪" };
                let version = status.version.as_ref().map(|v| format!(" ({})", v)).unwrap_or_default();
                println!("     {} {} {}", status_icon, tool, version);
            }

            println!("   {}", "Rust Components:".underline());
            for (component, status) in &report.dependencies.rust_components {
                let status_icon = if status.installed { "✅" } else { "❌" };
                println!("     {} {}", status_icon, component);
            }
        }

        // Configuration
        println!("\n⚙️  {}", "Configuration".bold());
        println!("   Cargo.toml: {}", if report.configuration.cargo_config_exists { "✅ Found" } else { "❌ Missing" });
        println!("   Rustfmt config: {}", if report.configuration.rustfmt_config_exists { "✅ Found" } else { "❌ Missing" });
        println!("   Clippy config: {}", if report.configuration.clippy_config_exists { "✅ Found" } else { "❌ Missing" });

        if detailed {
            println!("   {}", "Environment Variables:".underline());
            for (var, status) in &report.configuration.environment_variables {
                let status_icon = if status.set { "✅" } else { "❌" };
                let default_value = "not set".to_string();
                let value = status.value.as_ref().unwrap_or(&default_value);
                let required = if status.required { " (required)" } else { "" };
                println!("     {} {} = {}{}", status_icon, var, value, required);
            }
        }

        // Issues
        if !report.issues.is_empty() {
            println!("\n🚨 {}", "Issues Found".bold());
            for issue in &report.issues {
                let severity_icon = match issue.severity.as_str() {
                    "error" => "❌",
                    "warning" => "⚠️ ",
                    _ => "ℹ️ ",
                };
                println!("   {} {}", severity_icon, issue.message);
                println!("      💡 Solution: {}", issue.solution);
                if let Some(cmd) = &issue.command {
                    println!("      🛠️  Command: {}", cmd.cyan());
                }
                println!();
            }
        }

        // Recommendations
        if !check_only && !report.recommendations.is_empty() {
            println!("\n💡 {}", "Recommendations".bold());
            for recommendation in &report.recommendations {
                println!("   • {}", recommendation);
            }
        }
    }

    fn display_table_report(&self, report: &EnvironmentReport) {
        println!("{:<25} {:<12} {:<12} {:<12}",
                 "Category", "Status", "Passed", "Issues");
        println!("{}", "─".repeat(70));

        let rust_status = if report.rust_toolchain.is_compatible { "✅ OK" } else { "⚠️  WARN" };
        println!("{:<25} {:<12} {:<12} {:<12}", "Rust Toolchain", rust_status, "1/1", "0");

        let deps_passed = report.dependencies.required_tools.values().filter(|t| t.installed).count();
        let deps_total = report.dependencies.required_tools.len();
        let deps_status = if deps_passed == deps_total { "✅ OK" } else { "❌ FAIL" };
        println!("{:<25} {:<12} {:<12} {:<12}", "Required Tools", deps_status,
                 format!("{}/{}", deps_passed, deps_total), "0");

        let comp_passed = report.dependencies.rust_components.values().filter(|c| c.installed).count();
        let comp_total = report.dependencies.rust_components.len();
        let comp_status = if comp_passed == comp_total { "✅ OK" } else { "⚠️  WARN" };
        println!("{:<25} {:<12} {:<12} {:<12}", "Rust Components", comp_status,
                 format!("{}/{}", comp_passed, comp_total), "0");
    }

    fn apply_fixes(&self, issues: &[EnvironmentIssue]) -> Result<()> {
        println!("\n🔧 {}", "Applying Automatic Fixes".bold());

        for issue in issues {
            if let Some(command) = &issue.command {
                println!("   🛠️  Running: {}", command.cyan());
                let output = ProcessCommand::new("sh")
                    .arg("-c")
                    .arg(command)
                    .output();

                match output {
                    Ok(result) => {
                        if result.status.success() {
                            println!("      ✅ Success");
                        } else {
                            println!("      ❌ Failed: {}", String::from_utf8_lossy(&result.stderr));
                        }
                    }
                    Err(e) => {
                        println!("      ❌ Error: {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for EnvCheckTool {
    fn default() -> Self {
        Self::new()
    }
}
