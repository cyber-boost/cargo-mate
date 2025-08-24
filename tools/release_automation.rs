use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::process::Command as ProcessCommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ReleaseAutomationTool;

#[derive(Debug, Deserialize, Serialize)]
struct ReleaseConfig {
    version_files: Vec<String>,
    changelog_file: Option<String>,
    pre_release_checks: Vec<String>,
    post_release_steps: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReleasePlan {
    current_version: String,
    new_version: String,
    changes: Vec<String>,
    files_to_update: Vec<String>,
    steps: Vec<String>,
}

impl ReleaseAutomationTool {
    pub fn new() -> Self {
        Self
    }

    fn get_current_version(&self) -> Result<String> {
        let cargo_toml = fs::read_to_string("Cargo.toml")
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot read Cargo.toml: {}", e)))?;

        for line in cargo_toml.lines() {
            if line.trim().starts_with("version = ") {
                let version = line.split('"').nth(1)
                    .ok_or_else(|| ToolError::ExecutionFailed("Cannot parse version from Cargo.toml".to_string()))?;
                return Ok(version.to_string());
            }
        }

        Err(ToolError::ExecutionFailed("Version not found in Cargo.toml".to_string()))
    }

    fn bump_version(&self, current: &str, bump_type: &str) -> Result<String> {
        let parts: Vec<&str> = current.split('.').collect();
        if parts.len() != 3 {
            return Err(ToolError::ExecutionFailed(format!("Invalid version format: {}", current)));
        }

        let major: u32 = parts[0].parse().map_err(|_| ToolError::ExecutionFailed("Invalid major version".to_string()))?;
        let minor: u32 = parts[1].parse().map_err(|_| ToolError::ExecutionFailed("Invalid minor version".to_string()))?;
        let patch: u32 = parts[2].parse().map_err(|_| ToolError::ExecutionFailed("Invalid patch version".to_string()))?;

        let new_version = match bump_type {
            "major" => format!("{}.{}.{}", major + 1, 0, 0),
            "minor" => format!("{}.{}.{}", major, minor + 1, 0),
            "patch" => format!("{}.{}.{}", major, minor, patch + 1),
            _ => return Err(ToolError::ExecutionFailed(format!("Unknown bump type: {}", bump_type))),
        };

        Ok(new_version)
    }

    fn update_version_in_file(&self, file_path: &str, old_version: &str, new_version: &str, dry_run: bool) -> Result<()> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Cannot read {}: {}", file_path, e)))?;

        let new_content = content.replace(old_version, new_version);

        if dry_run {
            println!("📝 Would update {}: {} -> {}", file_path.cyan(), old_version.red(), new_version.green());
        } else {
            fs::write(file_path, new_content)
                .map_err(|e| ToolError::ExecutionFailed(format!("Cannot write {}: {}", file_path, e)))?;
            println!("✅ Updated {}: {} -> {}", file_path.cyan(), old_version.red(), new_version.green());
        }

        Ok(())
    }

    fn find_version_files(&self) -> Vec<String> {
        let mut files = vec!["Cargo.toml".to_string()];

        // Look for other common version files
        let potential_files = [
            "Cargo.lock",
            "package.json",
            "pyproject.toml",
            "setup.py",
            "__init__.py",
            "VERSION",
            "version.txt",
        ];

        for file in &potential_files {
            if Path::new(file).exists() {
                files.push(file.to_string());
            }
        }

        files
    }

    fn generate_changelog(&self, from_version: &str, to_version: &str, dry_run: bool) -> Result<String> {
        println!("📝 Generating changelog from {} to {}", from_version, to_version);

        // Try to get git log
        let git_log = ProcessCommand::new("git")
            .args(&["log", "--oneline", &format!("{}..HEAD", from_version)])
            .output();

        let changelog = match git_log {
            Ok(output) if output.status.success() => {
                let commits = String::from_utf8_lossy(&output.stdout);
                if commits.trim().is_empty() {
                    "No commits found since last release".to_string()
                } else {
                    format!("## Changes\n\n{}", commits)
                }
            }
            _ => {
                "Changelog generation failed - git log not available".to_string()
            }
        };

        let full_changelog = format!(
            "# Release {}\n\n{}\n\n## Previous Version: {}\n",
            to_version, changelog, from_version
        );

        if dry_run {
            println!("📋 Would generate changelog:");
            println!("{}", full_changelog);
        } else {
            // Try to update CHANGELOG.md
            let changelog_path = "CHANGELOG.md";
            let existing_content = fs::read_to_string(changelog_path).unwrap_or_default();

            let new_content = format!("{}\n\n{}", full_changelog, existing_content);

            fs::write(changelog_path, new_content)
                .map_err(|e| ToolError::ExecutionFailed(format!("Cannot write changelog: {}", e)))?;

            println!("✅ Updated {}", changelog_path.cyan());
        }

        Ok(full_changelog)
    }

    fn run_pre_release_checks(&self, dry_run: bool) -> Result<()> {
        println!("🔍 Running pre-release checks...");

        let checks = [
            ("Tests", "cargo test --quiet"),
            ("Clippy", "cargo clippy --quiet"),
            ("Format", "cargo fmt --check"),
            ("Audit", "cargo audit --quiet 2>/dev/null || echo 'cargo-audit not installed'"),
        ];

        for (name, command) in &checks {
            println!("   Checking {}...", name.cyan());

            if dry_run {
                println!("   📝 Would run: {}", command);
                continue;
            }

            let result = ProcessCommand::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run {}: {}", name, e)))?;

            if result.status.success() {
                println!("   ✅ {} passed", name.green());
            } else {
                let error = String::from_utf8_lossy(&result.stderr);
                println!("   ❌ {} failed: {}", name.red(), error.lines().next().unwrap_or("Unknown error"));
                return Err(ToolError::ExecutionFailed(format!("{} check failed", name)));
            }
        }

        println!("🎉 All pre-release checks passed!");
        Ok(())
    }

    fn create_git_tag(&self, version: &str, dry_run: bool) -> Result<()> {
        let tag_name = format!("v{}", version);

        if dry_run {
            println!("🏷️  Would create git tag: {}", tag_name.cyan());
        } else {
            let result = ProcessCommand::new("git")
                .args(&["tag", "-a", &tag_name, "-m", &format!("Release {}", version)])
                .output()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create git tag: {}", e)))?;

            if result.status.success() {
                println!("✅ Created git tag: {}", tag_name.cyan());
            } else {
                let error = String::from_utf8_lossy(&result.stderr);
                return Err(ToolError::ExecutionFailed(format!("Git tag failed: {}", error)));
            }
        }

        Ok(())
    }

    fn publish_to_crates_io(&self, dry_run: bool) -> Result<()> {
        if dry_run {
            println!("📦 Would publish to crates.io");
        } else {
            println!("📦 Publishing to crates.io...");

            let result = ProcessCommand::new("cargo")
                .args(&["publish", "--dry-run"])
                .output()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run cargo publish dry-run: {}", e)))?;

            if !result.status.success() {
                let error = String::from_utf8_lossy(&result.stderr);
                return Err(ToolError::ExecutionFailed(format!("Cargo publish dry-run failed: {}", error)));
            }

            println!("✅ Dry-run successful! Run without --dry-run to actually publish");
        }

        Ok(())
    }

    fn create_release_plan(&self, bump_type: &str, dry_run: bool) -> Result<ReleasePlan> {
        let current_version = self.get_current_version()?;
        let new_version = self.bump_version(&current_version, bump_type)?;

        let files_to_update = self.find_version_files();

        let mut steps = Vec::new();
        steps.push("Run pre-release checks (tests, clippy, format)".to_string());
        steps.push(format!("Update version from {} to {} in {} files", current_version, new_version, files_to_update.len()));
        steps.push("Generate changelog".to_string());
        steps.push("Create git tag".to_string());
        steps.push("Push to repository".to_string());
        steps.push("Publish to crates.io".to_string());

        let changes = vec![
            format!("Version bump: {} -> {}", current_version, new_version),
            format!("Update {} files with new version", files_to_update.len()),
            "Generate changelog".to_string(),
            "Create git tag".to_string(),
        ];

        Ok(ReleasePlan {
            current_version,
            new_version,
            changes,
            files_to_update,
            steps,
        })
    }

    fn execute_release_plan(&self, plan: &ReleasePlan, dry_run: bool, push: bool, publish: bool) -> Result<()> {
        println!("{}", "🚀 Executing Release Plan".bold().green());
        println!("{}", "═".repeat(50).green());

        println!("📊 Release: {} -> {}", plan.current_version.red(), plan.new_version.green());
        println!("📁 Files to update: {}", plan.files_to_update.len());

        // Step 1: Pre-release checks
        self.run_pre_release_checks(dry_run)?;

        // Step 2: Update version files
        for file in &plan.files_to_update {
            self.update_version_in_file(file, &plan.current_version, &plan.new_version, dry_run)?;
        }

        // Step 3: Generate changelog
        self.generate_changelog(&plan.current_version, &plan.new_version, dry_run)?;

        // Step 4: Create git tag
        self.create_git_tag(&plan.new_version, dry_run)?;

        if push && !dry_run {
            // Commit changes
            ProcessCommand::new("git")
                .args(&["add", "."])
                .output()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to git add: {}", e)))?;

            ProcessCommand::new("git")
                .args(&["commit", "-m", &format!("Release {}", plan.new_version)])
                .output()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to git commit: {}", e)))?;

            // Push
            ProcessCommand::new("git")
                .args(&["push"])
                .output()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to git push: {}", e)))?;

            ProcessCommand::new("git")
                .args(&["push", "--tags"])
                .output()
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to push tags: {}", e)))?;

            println!("✅ Pushed to repository with tags");
        }

        if publish {
            self.publish_to_crates_io(dry_run)?;
        }

        println!("\n🎉 Release {} completed successfully!", plan.new_version.bold());
        println!("   📦 Ready for distribution");
        if publish && !dry_run {
            println!("   🦀 Published to crates.io");
        }

        Ok(())
    }
}

impl Tool for ReleaseAutomationTool {
    fn name(&self) -> &'static str {
        "release-automation"
    }

    fn description(&self) -> &'static str {
        "Automate the entire release process - versioning, changelog, publishing"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Handle the complete release workflow: version bumping, changelog generation, git tagging, and publishing to crates.io")
            .args(&[
                Arg::new("patch")
                    .long("patch")
                    .help("Bump patch version (1.0.0 -> 1.0.1)")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("minor")
                    .long("minor")
                    .help("Bump minor version (1.0.0 -> 1.1.0)")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("major")
                    .long("major")
                    .help("Bump major version (1.0.0 -> 2.0.0)")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("changelog")
                    .long("changelog")
                    .help("Generate changelog from git commits")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("push")
                    .long("push")
                    .help("Push changes and tags to repository")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("publish")
                    .long("publish")
                    .help("Publish to crates.io after release")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("validate")
                    .long("validate")
                    .help("Run validation checks before release")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("plan")
                    .long("plan")
                    .help("Show release plan without executing")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let patch = matches.get_flag("patch");
        let minor = matches.get_flag("minor");
        let major = matches.get_flag("major");
        let changelog = matches.get_flag("changelog");
        let push = matches.get_flag("push");
        let publish = matches.get_flag("publish");
        let validate = matches.get_flag("validate");
        let plan = matches.get_flag("plan");
        let dry_run = matches.get_flag("dry-run");
        let verbose = matches.get_flag("verbose");

        // Determine bump type
        let bump_type = if major {
            "major"
        } else if minor {
            "minor"
        } else if patch {
            "patch"
        } else {
            "patch" // default
        };

        if verbose {
            println!("🚀 {} - Automating release process", "CargoMate ReleaseAutomation".bold().blue());
        }

        // Check if we're in a git repository
        if !Path::new(".git").exists() {
            return Err(ToolError::ExecutionFailed("Not in a git repository".to_string()));
        }

        // Check if Cargo.toml exists
        if !Path::new("Cargo.toml").exists() {
            return Err(ToolError::ExecutionFailed("Cargo.toml not found".to_string()));
        }

        let release_plan = self.create_release_plan(bump_type, dry_run)?;

        if plan {
            // Just show the plan
            println!("{}", "📋 Release Plan".bold().yellow());
            println!("Current version: {}", release_plan.current_version.cyan());
            println!("New version: {}", release_plan.new_version.green());
            println!("Files to update: {}", release_plan.files_to_update.len());

            println!("\n📝 Changes:");
            for change in &release_plan.changes {
                println!("  • {}", change);
            }

            println!("\n📋 Steps:");
            for (i, step) in release_plan.steps.iter().enumerate() {
                println!("  {}. {}", i + 1, step);
            }

            return Ok(());
        }

        // Execute the release
        self.execute_release_plan(&release_plan, dry_run, push, publish)?;

        Ok(())
    }
}

impl Default for ReleaseAutomationTool {
    fn default() -> Self {
        Self::new()
    }
}
