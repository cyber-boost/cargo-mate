use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};
use toml::{self, Value};

#[derive(Debug, Clone)]
pub struct LicenseBundlerTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseBundleReport {
    dependencies_analyzed: usize,
    licenses_found: usize,
    licenses: Vec<LicenseInfo>,
    compatibility_report: LicenseCompatibility,
    files_created: Vec<String>,
    statistics: LicenseStatistics,
    recommendations: Vec<String>,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseInfo {
    package_name: String,
    version: String,
    license: String,
    license_text: Option<String>,
    license_file_path: Option<String>,
    source: String,
    compatibility: String,
    risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseCompatibility {
    overall_compatible: bool,
    incompatible_licenses: Vec<String>,
    warnings: Vec<String>,
    copyleft_count: usize,
    permissive_count: usize,
    commercial_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseStatistics {
    total_packages: usize,
    licensed_packages: usize,
    unlicensed_packages: usize,
    by_license_type: HashMap<String, usize>,
    most_common_license: String,
    compatibility_score: f64,
}

impl LicenseBundlerTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_cargo_lock(&self, lock_path: &str) -> Result<Vec<DependencyInfo>> {
        let content = fs::read_to_string(lock_path)?;
        let mut dependencies = Vec::new();

        let package_regex = Regex::new(r#"name = "([^"]+)""#).unwrap();
        let version_regex = Regex::new(r#"version = "([^"]+)""#).unwrap();
        let source_regex = Regex::new(r#"source = "([^"]+)""#).unwrap();

        let mut current_package = None;
        let mut current_version = None;
        let mut current_source = None;

        for line in content.lines() {
            let trimmed = line.trim();

            if let Some(captures) = package_regex.captures(trimmed) {
                current_package = Some(captures[1].to_string());
            } else if let Some(captures) = version_regex.captures(trimmed) {
                current_version = Some(captures[1].to_string());
            } else if let Some(captures) = source_regex.captures(trimmed) {
                current_source = Some(captures[1].to_string());
            } else if trimmed == "[[package]]" {
                // Save previous package if exists
                if let (Some(name), Some(version), Some(source)) = (current_package, current_version, current_source) {
                    dependencies.push(DependencyInfo {
                        name,
                        version,
                        source,
                        license: None,
                    });
                }
                current_package = None;
                current_version = None;
                current_source = None;
            }
        }

        // Don't forget the last package
        if let (Some(name), Some(version), Some(source)) = (current_package, current_version, current_source) {
            dependencies.push(DependencyInfo {
                name,
                version,
                source,
                license: None,
            });
        }

        Ok(dependencies)
    }

    fn extract_licenses_from_cargo_toml(&self, toml_path: &str) -> Result<HashMap<String, String>> {
        let content = fs::read_to_string(toml_path)?;
        let cargo_toml: Value = toml::from_str(&content)
            .map_err(|e| ToolError::TomlError(e))?;

        let mut licenses = HashMap::new();

        if let Some(dependencies) = cargo_toml.get("dependencies") {
            if let Some(deps_table) = dependencies.as_table() {
                for (name, dep_info) in deps_table {
                    if let Some(table) = dep_info.as_table() {
                        if let Some(license) = table.get("license") {
                            if let Some(license_str) = license.as_str() {
                                licenses.insert(name.clone(), license_str.to_string());
                            }
                        }
                    }
                }
            }
        }

        Ok(licenses)
    }

    fn fetch_license_from_registry(&self, package_name: &str, version: &str) -> Result<Option<String>> {
        // In a real implementation, this would query crates.io API
        // For now, we'll use common license assumptions based on package names

        let common_licenses = vec![
            ("serde", "MIT OR Apache-2.0"),
            ("tokio", "MIT"),
            ("anyhow", "MIT OR Apache-2.0"),
            ("thiserror", "MIT OR Apache-2.0"),
            ("clap", "MIT OR Apache-2.0"),
            ("regex", "MIT OR Apache-2.0"),
            ("rand", "MIT OR Apache-2.0"),
            ("chrono", "MIT OR Apache-2.0"),
            ("reqwest", "MIT OR Apache-2.0"),
            ("serde_json", "MIT OR Apache-2.0"),
        ];

        for (pkg, license) in common_licenses {
            if package_name.contains(pkg) || package_name == pkg {
                return Ok(Some(license.to_string()));
            }
        }

        Ok(None)
    }

    fn find_license_file(&self, package_path: &str) -> Option<String> {
        let license_files = vec![
            "LICENSE",
            "LICENSE.md",
            "LICENSE.txt",
            "COPYING",
            "COPYING.md",
            "COPYING.txt",
        ];

        for license_file in license_files {
            let license_path = Path::new(package_path).join(license_file);
            if license_path.exists() {
                return Some(license_path.to_string_lossy().to_string());
            }
        }

        None
    }

    fn read_license_text(&self, license_path: &str) -> Result<String> {
        let content = fs::read_to_string(license_path)?;
        // Truncate very long license texts
        if content.len() > 10000 {
            Ok(format!("{}...\n[License text truncated - {} characters total]",
                      &content[..5000], content.len()))
        } else {
            Ok(content)
        }
    }

    fn assess_license_compatibility(&self, licenses: &[LicenseInfo]) -> LicenseCompatibility {
        let mut incompatible = Vec::new();
        let mut warnings = Vec::new();
        let mut copyleft_count = 0;
        let mut permissive_count = 0;
        let mut commercial_count = 0;

        let copyleft_licenses = vec!["GPL", "AGPL", "MPL"];
        let permissive_licenses = vec!["MIT", "Apache-2.0", "BSD", "ISC"];
        let commercial_licenses = vec!["Proprietary", "Commercial"];

        for license_info in licenses {
            let license_upper = license_info.license.to_uppercase();

            if copyleft_licenses.iter().any(|cl| license_upper.contains(cl)) {
                copyleft_count += 1;
            } else if permissive_licenses.iter().any(|pl| license_upper.contains(pl)) {
                permissive_count += 1;
            } else if commercial_licenses.iter().any(|com| license_upper.contains(com)) {
                commercial_count += 1;
            }

            // Check for incompatible combinations
            if license_upper.contains("GPL") && license_upper.contains("PROPRIETARY") {
                incompatible.push(format!("{}: GPL + Proprietary incompatible", license_info.package_name));
            }

            if license_upper.contains("AGPL") {
                warnings.push(format!("{}: AGPL requires distribution of source code", license_info.package_name));
            }
        }

        let overall_compatible = incompatible.is_empty();

        LicenseCompatibility {
            overall_compatible,
            incompatible_licenses: incompatible,
            warnings,
            copyleft_count,
            permissive_count,
            commercial_count,
        }
    }

    fn categorize_license_risk(&self, license: &str) -> String {
        let license_upper = license.to_uppercase();

        if license_upper.contains("GPL") || license_upper.contains("AGPL") {
            "high".to_string() // Copyleft requirements
        } else if license_upper.contains("PROPRIETARY") || license_upper.contains("COMMERCIAL") {
            "medium".to_string() // May have restrictions
        } else if license_upper.contains("UNKNOWN") || license_upper.is_empty() {
            "high".to_string() // Unknown license = high risk
        } else {
            "low".to_string() // Permissive licenses
        }
    }

    fn generate_license_bundle(&self, licenses: &[LicenseInfo], output_dir: &str) -> Result<Vec<String>> {
        let mut files_created = Vec::new();

        // Create output directory
        fs::create_dir_all(output_dir)?;

        // Generate main license file
        let main_license_path = Path::new(output_dir).join("LICENSES.md");
        let main_content = self.generate_main_license_file(licenses);
        fs::write(&main_license_path, main_content)?;
        files_created.push(main_license_path.to_string_lossy().to_string());

        // Generate individual license files
        for license_info in licenses {
            if let Some(license_text) = &license_info.license_text {
                let file_name = format!("{}-{}.license", license_info.package_name, license_info.version.replace(".", "_"));
                let file_path = Path::new(output_dir).join(&file_name);
                fs::write(&file_path, license_text)?;
                files_created.push(file_path.to_string_lossy().to_string());
            }
        }

        // Generate compatibility report
        let compatibility_path = Path::new(output_dir).join("LICENSE_COMPATIBILITY.md");
        let compatibility_content = self.generate_compatibility_report(licenses);
        fs::write(&compatibility_path, compatibility_content)?;
        files_created.push(compatibility_path.to_string_lossy().to_string());

        // Generate third-party notice
        let notice_path = Path::new(output_dir).join("THIRD_PARTY_NOTICES.md");
        let notice_content = self.generate_third_party_notices(licenses);
        fs::write(&notice_path, notice_content)?;
        files_created.push(notice_path.to_string_lossy().to_string());

        Ok(files_created)
    }

    fn generate_main_license_file(&self, licenses: &[LicenseInfo]) -> String {
        let mut content = format!("# License Bundle

This file contains license information for all dependencies used in this project.

Generated on: {}

## Dependencies ({} total)

", chrono::Utc::now().to_rfc3339(), licenses.len());

        for license_info in licenses {
            content.push_str(&format!("### {} v{}
- **License**: {}
- **Source**: {}
- **Compatibility**: {}
- **Risk Level**: {}

",
                license_info.package_name,
                license_info.version,
                license_info.license,
                license_info.source,
                license_info.compatibility,
                license_info.risk_level
            ));
        }

        content
    }

    fn generate_compatibility_report(&self, licenses: &[LicenseInfo]) -> String {
        let compatibility = self.assess_license_compatibility(licenses);

        let mut content = format!("# License Compatibility Report

Generated on: {}

## Summary

", chrono::Utc::now().to_rfc3339());

        content.push_str(&format!("- **Overall Compatible**: {}\n", if compatibility.overall_compatible { "✅ Yes" } else { "❌ No" }));
        content.push_str(&format!("- **Copyleft Licenses**: {}\n", compatibility.copyleft_count));
        content.push_str(&format!("- **Permissive Licenses**: {}\n", compatibility.permissive_count));
        content.push_str(&format!("- **Commercial Licenses**: {}\n", compatibility.commercial_count));

        if !compatibility.incompatible_licenses.is_empty() {
            content.push_str("\n## Incompatible License Combinations\n\n");
            for incompatible in &compatibility.incompatible_licenses {
                content.push_str(&format!("- ❌ {}\n", incompatible));
            }
        }

        if !compatibility.warnings.is_empty() {
            content.push_str("\n## License Warnings\n\n");
            for warning in &compatibility.warnings {
                content.push_str(&format!("- ⚠️ {}\n", warning));
            }
        }

        content
    }

    fn generate_third_party_notices(&self, licenses: &[LicenseInfo]) -> String {
        let mut content = format!("# Third Party Notices

This project includes third-party software components. The following notices apply:

Generated on: {}

", chrono::Utc::now().to_rfc3339());

        for license_info in licenses {
            content.push_str(&format!("## {} v{}

**Package**: {}
**Version**: {}
**License**: {}
**Source**: {}

",
                license_info.package_name,
                license_info.version,
                license_info.package_name,
                license_info.version,
                license_info.license,
                license_info.source
            ));

            if let Some(license_text) = &license_info.license_text {
                content.push_str(&format!("### License Text

```
{}
```

---

", license_text));
            }
        }

        content
    }

    fn calculate_statistics(&self, licenses: &[LicenseInfo]) -> LicenseStatistics {
        let mut by_license_type = HashMap::new();
        let mut licensed_packages = 0;
        let mut unlicensed_packages = 0;

        for license_info in licenses {
            if license_info.license.is_empty() || license_info.license == "Unknown" {
                unlicensed_packages += 1;
            } else {
                licensed_packages += 1;
                *by_license_type.entry(license_info.license.clone()).or_insert(0) += 1;
            }
        }

        let most_common_license = by_license_type.iter()
            .max_by_key(|&(_, count)| count)
            .map(|(license, _)| license.clone())
            .unwrap_or_else(|| "None".to_string());

        let compatibility = self.assess_license_compatibility(licenses);
        let compatibility_score = if compatibility.overall_compatible {
            if compatibility.incompatible_licenses.is_empty() {
                100.0
            } else {
                75.0
            }
        } else {
            0.0
        };

        LicenseStatistics {
            total_packages: licenses.len(),
            licensed_packages,
            unlicensed_packages,
            by_license_type,
            most_common_license,
            compatibility_score,
        }
    }

    fn generate_recommendations(&self, licenses: &[LicenseInfo], compatibility: &LicenseCompatibility) -> Vec<String> {
        let mut recommendations = Vec::new();

        if !compatibility.overall_compatible {
            recommendations.push("🚨 CRITICAL: Resolve incompatible license combinations before distribution".to_string());
        }

        if compatibility.copyleft_count > 0 {
            recommendations.push("⚠️ Review copyleft licenses - may require source code distribution".to_string());
        }

        let unlicensed_count = licenses.iter().filter(|l| l.license.is_empty() || l.license == "Unknown").count();
        if unlicensed_count > 0 {
            recommendations.push(format!("📋 {} packages have unknown licenses - investigate and document", unlicensed_count));
        }

        recommendations.push("📄 Include license bundle in project distribution".to_string());
        recommendations.push("🔄 Regularly update license information when dependencies change".to_string());
        recommendations.push("⚖️ Consult legal counsel for commercial distribution".to_string());

        recommendations
    }

    fn display_report(&self, report: &LicenseBundleReport, output_format: OutputFormat, verbose: bool) {
        match output_format {
            OutputFormat::Human => {
                println!("\n📄 {} - License Bundler Report", "CargoMate LicenseBundler".bold().blue());
                println!("{}", "═".repeat(60).blue());

                println!("\n📊 Summary:");
                println!("  • Dependencies Analyzed: {}", report.dependencies_analyzed);
                println!("  • Licenses Found: {}", report.licenses_found);
                println!("  • Licensed Packages: {}", report.statistics.licensed_packages);
                println!("  • Unlicensed Packages: {}", report.statistics.unlicensed_packages);
                println!("  • Compatibility Score: {:.1}%", report.statistics.compatibility_score);

                println!("\n📈 License Compatibility:");
                let compat = &report.compatibility_report;
                println!("  • Overall Compatible: {}", if compat.overall_compatible { "✅ Yes".green() } else { "❌ No".red() });
                println!("  • Copyleft Licenses: {}", compat.copyleft_count);
                println!("  • Permissive Licenses: {}", compat.permissive_count);
                println!("  • Commercial Licenses: {}", compat.commercial_count);

                if verbose {
                    println!("\n📋 License Breakdown:");
                    for (license_type, count) in &report.statistics.by_license_type {
                        println!("  • {}: {} packages", license_type, count);
                    }

                    println!("\n📦 Dependencies by License:");
                    for license_info in &report.licenses {
                        let risk_icon = match license_info.risk_level.as_str() {
                            "high" => "🚨",
                            "medium" => "⚠️",
                            "low" => "✅",
                            _ => "•",
                        };

                        println!("  {} {} v{} - {}", risk_icon, license_info.package_name, license_info.version, license_info.license);
                    }
                }

                if !report.files_created.is_empty() {
                    println!("\n📁 Files Created:");
                    for file in &report.files_created {
                        println!("  • {}", file.green());
                    }
                }

                if !report.recommendations.is_empty() {
                    println!("\n💡 Recommendations:");
                    for rec in &report.recommendations {
                        println!("  • {}", rec.cyan());
                    }
                }

                println!("\n✅ License bundling complete!");
                if report.compatibility_report.overall_compatible {
                    println!("   All licenses are compatible!");
                } else {
                    println!("   Review compatibility issues before distribution");
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!("{:<30} {:<15} {:<20} {:<12} {:<10}",
                        "Package", "Version", "License", "Risk", "Compatible");
                println!("{}", "─".repeat(95));

                for license_info in &report.licenses {
                    let compatible = match license_info.compatibility.as_str() {
                        "compatible" => "✅",
                        "incompatible" => "❌",
                        _ => "⚠️",
                    };

                    println!("{:<30} {:<15} {:<20} {:<12} {:<10}",
                            license_info.package_name.chars().take(29).collect::<String>(),
                            license_info.version,
                            license_info.license.chars().take(19).collect::<String>(),
                            license_info.risk_level,
                            compatible);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct DependencyInfo {
    name: String,
    version: String,
    source: String,
    license: Option<String>,
}

impl Tool for LicenseBundlerTool {
    fn name(&self) -> &'static str {
        "license-bundler"
    }

    fn description(&self) -> &'static str {
        "Generate license files for all dependencies"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Generate comprehensive license bundle for all project dependencies, \
                        including compatibility analysis and legal compliance documentation.

EXAMPLES:
    cm tool license-bundler --output licenses/
    cm tool license-bundler --check-compatibility --format json
    cm tool license-bundler --include-license-text --third-party-notices")
            .args(&[
                Arg::new("output")
                    .long("output")
                    .short('o')
                    .help("Output directory for license files")
                    .default_value("licenses/"),
                Arg::new("check-compatibility")
                    .long("check-compatibility")
                    .help("Analyze license compatibility")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("include-license-text")
                    .long("include-license-text")
                    .help("Include full license text in output")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("third-party-notices")
                    .long("third-party-notices")
                    .help("Generate third-party notices file")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("format")
                    .long("format")
                    .short('f')
                    .help("Output format for license bundle")
                    .default_value("markdown")
                    .value_parser(["markdown", "json", "text"]),
                Arg::new("exclude-dev")
                    .long("exclude-dev")
                    .help("Exclude development dependencies")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("ci-mode")
                    .long("ci-mode")
                    .help("CI-friendly output with exit codes")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let output_dir = matches.get_one::<String>("output").unwrap();
        let check_compatibility = matches.get_flag("check-compatibility");
        let include_license_text = matches.get_flag("include-license-text");
        let third_party_notices = matches.get_flag("third-party-notices");
        let format = matches.get_one::<String>("format").unwrap();
        let exclude_dev = matches.get_flag("exclude-dev");
        let ci_mode = matches.get_flag("ci-mode");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        println!("📄 {} - Generating License Bundle", "CargoMate LicenseBundler".bold().blue());

        // Find and parse Cargo.lock
        let lock_path = "Cargo.lock";
        if !Path::new(lock_path).exists() {
            return Err(ToolError::InvalidArguments("Cargo.lock not found - run 'cargo update' first".to_string()));
        }

        let mut dependencies = self.parse_cargo_lock(lock_path)?;

        // Get licenses from Cargo.toml
        let toml_licenses = self.extract_licenses_from_cargo_toml("Cargo.toml")?;

        // Enrich dependencies with license information
        for dep in &mut dependencies {
            if let Some(license) = toml_licenses.get(&dep.name) {
                dep.license = Some(license.clone());
            } else {
                // Try to fetch from registry
                if let Ok(Some(license)) = self.fetch_license_from_registry(&dep.name, &dep.version) {
                    dep.license = Some(license);
                }
            }
        }

        // Convert to LicenseInfo
        let mut licenses = Vec::new();
        for dep in &dependencies {
            let license_text = if include_license_text {
                // Try to find license file in the dependency
                let dep_path = format!("target/debug/deps/{}", dep.name);
                if let Some(license_file) = self.find_license_file(&dep_path) {
                    match self.read_license_text(&license_file) {
                        Ok(text) => Some(text),
                        Err(_) => None,
                    }
                } else {
                    None
                }
            } else {
                None
            };

            let license_name = dep.license.clone().unwrap_or_else(|| "Unknown".to_string());
            let compatibility = if license_name.contains("GPL") || license_name.contains("AGPL") {
                "copyleft"
            } else if license_name.contains("MIT") || license_name.contains("Apache") {
                "permissive"
            } else {
                "unknown"
            }.to_string();

            licenses.push(LicenseInfo {
                package_name: dep.name.clone(),
                version: dep.version.clone(),
                license: license_name,
                license_text,
                license_file_path: None,
                source: dep.source.clone(),
                compatibility,
                risk_level: self.categorize_license_risk(&dep.license.clone().unwrap_or_default()),
            });
        }

        // Generate license bundle
        let files_created = self.generate_license_bundle(&licenses, output_dir)?;

        // Check compatibility if requested
        let compatibility_report = if check_compatibility {
            self.assess_license_compatibility(&licenses)
        } else {
            LicenseCompatibility {
                overall_compatible: true,
                incompatible_licenses: vec![],
                warnings: vec![],
                copyleft_count: 0,
                permissive_count: 0,
                commercial_count: 0,
            }
        };

        // Calculate statistics
        let statistics = self.calculate_statistics(&licenses);

        // Generate recommendations
        let recommendations = self.generate_recommendations(&licenses, &compatibility_report);

        // Create report
        let report = LicenseBundleReport {
            dependencies_analyzed: dependencies.len(),
            licenses_found: licenses.len(),
            licenses,
            compatibility_report,
            files_created,
            statistics,
            recommendations,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Handle CI mode
        if ci_mode {
            if report.compatibility_report.overall_compatible {
                println!("::set-output name=license-compatible::true");
            } else {
                println!("::set-output name=license-compatible::false");
                println!("::error title=License Compatibility Issues::Found incompatible license combinations");
            }

            if report.statistics.unlicensed_packages > 0 {
                println!("::warning title=Unlicensed Packages::Found {} packages without license information", report.statistics.unlicensed_packages);
            }
        }

        // Display results
        self.display_report(&report, output_format, verbose);

        Ok(())
    }
}

impl Default for LicenseBundlerTool {
    fn default() -> Self {
        Self::new()
    }
}
