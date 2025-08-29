use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use toml::{map::Map, Value};
use crate::captain;
use clap::Subcommand;
#[derive(Subcommand, Debug)]
pub enum OptimizeAction {
    Aggressive,
    Balanced,
    Conservative,
    Custom {
        #[arg(default_value = "4")]
        jobs: u32,
        #[arg(default_value = "true")]
        incremental: String,
        #[arg(default_value = "1")]
        opt_level: u32,
        #[arg(default_value = "1")]
        debug_level: u32,
        #[arg(default_value = "128")]
        codegen_units: u32,
    },
    Status,
    Recommendations,
    Restore,
}
pub struct BuildOptimizer {
    project_root: PathBuf,
}
impl BuildOptimizer {
    pub fn new(project_root: Option<PathBuf>) -> Result<Self> {
        let project_root = project_root
            .unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            });
        Ok(Self { project_root })
    }
    pub fn optimize_build(&self, profile: OptimizationProfile) -> Result<()> {
        let cargo_toml_path = self.project_root.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Err(
                anyhow::anyhow!(
                    "Cargo.toml not found in {}", self.project_root.display()
                ),
            );
        }
        let content = fs::read_to_string(&cargo_toml_path)?;
        let mut config: Value = toml::from_str(&content)?;
        match profile {
            OptimizationProfile::Aggressive => {
                self.apply_aggressive_optimizations(&mut config)?
            }
            OptimizationProfile::Balanced => {
                self.apply_balanced_optimizations(&mut config)?
            }
            OptimizationProfile::Conservative => {
                self.apply_conservative_optimizations(&mut config)?
            }
            OptimizationProfile::Custom {
                jobs,
                incremental,
                opt_level,
                debug_level,
                codegen_units,
            } => {
                self.apply_custom_optimizations(
                    &mut config,
                    jobs,
                    incremental,
                    opt_level,
                    debug_level,
                    codegen_units,
                )?;
            }
        }
        let backup_path = cargo_toml_path.with_extension("toml.backup");
        fs::copy(&cargo_toml_path, &backup_path)?;
        println!("📋 Backed up Cargo.toml to {}", backup_path.display());
        let optimized_content = toml::to_string_pretty(&config)?;
        fs::write(&cargo_toml_path, optimized_content)?;
        println!("✅ Applied {} optimizations to Cargo.toml", profile.to_string());
        self.show_optimization_summary(&config)?;
        Ok(())
    }
    fn apply_aggressive_optimizations(&self, config: &mut Value) -> Result<()> {
        if !config.as_table().unwrap().contains_key("build") {
            config
                .as_table_mut()
                .unwrap()
                .insert("build".to_string(), Value::Table(Map::new()));
        }
        let build = config.get_mut("build").unwrap().as_table_mut().unwrap();
        build.insert("jobs".to_string(), Value::Integer(8));
        build.insert("incremental".to_string(), Value::Boolean(true));
        if !config.as_table().unwrap().contains_key("profile") {
            config
                .as_table_mut()
                .unwrap()
                .insert("profile".to_string(), Value::Table(Map::new()));
        }
        let profile = config.get_mut("profile").unwrap().as_table_mut().unwrap();
        if !profile.contains_key("dev") {
            profile.insert("dev".to_string(), Value::Table(Map::new()));
        }
        let dev = profile.get_mut("dev").unwrap().as_table_mut().unwrap();
        dev.insert("opt-level".to_string(), Value::Integer(1));
        dev.insert("debug".to_string(), Value::Integer(1));
        dev.insert("codegen-units".to_string(), Value::Integer(256));
        dev.insert("lto".to_string(), Value::Boolean(false));
        if !config.as_table().unwrap().contains_key("env") {
            config
                .as_table_mut()
                .unwrap()
                .insert("env".to_string(), Value::Table(Map::new()));
        }
        let env = config.get_mut("env").unwrap().as_table_mut().unwrap();
        env.insert("CARGO_INCREMENTAL".to_string(), Value::String("1".to_string()));
        env.insert("CARGO_BUILD_JOBS".to_string(), Value::String("8".to_string()));
        Ok(())
    }
    fn apply_balanced_optimizations(&self, config: &mut Value) -> Result<()> {
        if !config.as_table().unwrap().contains_key("build") {
            config
                .as_table_mut()
                .unwrap()
                .insert("build".to_string(), Value::Table(Map::new()));
        }
        let build = config.get_mut("build").unwrap().as_table_mut().unwrap();
        build.insert("jobs".to_string(), Value::Integer(4));
        build.insert("incremental".to_string(), Value::Boolean(true));
        if !config.as_table().unwrap().contains_key("profile") {
            config
                .as_table_mut()
                .unwrap()
                .insert("profile".to_string(), Value::Table(Map::new()));
        }
        let profile = config.get_mut("profile").unwrap().as_table_mut().unwrap();
        if !profile.contains_key("dev") {
            profile.insert("dev".to_string(), Value::Table(Map::new()));
        }
        let dev = profile.get_mut("dev").unwrap().as_table_mut().unwrap();
        dev.insert("opt-level".to_string(), Value::Integer(1));
        dev.insert("debug".to_string(), Value::Integer(1));
        dev.insert("codegen-units".to_string(), Value::Integer(128));
        dev.insert("lto".to_string(), Value::Boolean(false));
        if !config.as_table().unwrap().contains_key("env") {
            config
                .as_table_mut()
                .unwrap()
                .insert("env".to_string(), Value::Table(Map::new()));
        }
        let env = config.get_mut("env").unwrap().as_table_mut().unwrap();
        env.insert("CARGO_INCREMENTAL".to_string(), Value::String("1".to_string()));
        env.insert("CARGO_BUILD_JOBS".to_string(), Value::String("4".to_string()));
        Ok(())
    }
    fn apply_conservative_optimizations(&self, config: &mut Value) -> Result<()> {
        if !config.as_table().unwrap().contains_key("build") {
            config
                .as_table_mut()
                .unwrap()
                .insert("build".to_string(), Value::Table(Map::new()));
        }
        let build = config.get_mut("build").unwrap().as_table_mut().unwrap();
        build.insert("jobs".to_string(), Value::Integer(2));
        build.insert("incremental".to_string(), Value::Boolean(true));
        if !config.as_table().unwrap().contains_key("profile") {
            config
                .as_table_mut()
                .unwrap()
                .insert("profile".to_string(), Value::Table(Map::new()));
        }
        let profile = config.get_mut("profile").unwrap().as_table_mut().unwrap();
        if !profile.contains_key("dev") {
            profile.insert("dev".to_string(), Value::Table(Map::new()));
        }
        let dev = profile.get_mut("dev").unwrap().as_table_mut().unwrap();
        dev.insert("opt-level".to_string(), Value::Integer(0));
        dev.insert("debug".to_string(), Value::Integer(2));
        dev.insert("codegen-units".to_string(), Value::Integer(64));
        dev.insert("lto".to_string(), Value::Boolean(false));
        if !config.as_table().unwrap().contains_key("env") {
            config
                .as_table_mut()
                .unwrap()
                .insert("env".to_string(), Value::Table(Map::new()));
        }
        let env = config.get_mut("env").unwrap().as_table_mut().unwrap();
        env.insert("CARGO_INCREMENTAL".to_string(), Value::String("1".to_string()));
        env.insert("CARGO_BUILD_JOBS".to_string(), Value::String("2".to_string()));
        Ok(())
    }
    fn apply_custom_optimizations(
        &self,
        config: &mut Value,
        jobs: u32,
        incremental: bool,
        opt_level: u32,
        debug_level: u32,
        codegen_units: u32,
    ) -> Result<()> {
        if !config.as_table().unwrap().contains_key("build") {
            config
                .as_table_mut()
                .unwrap()
                .insert("build".to_string(), Value::Table(Map::new()));
        }
        let build = config.get_mut("build").unwrap().as_table_mut().unwrap();
        build.insert("jobs".to_string(), Value::Integer(jobs as i64));
        build.insert("incremental".to_string(), Value::Boolean(incremental));
        if !config.as_table().unwrap().contains_key("profile") {
            config
                .as_table_mut()
                .unwrap()
                .insert("profile".to_string(), Value::Table(Map::new()));
        }
        let profile = config.get_mut("profile").unwrap().as_table_mut().unwrap();
        if !profile.contains_key("dev") {
            profile.insert("dev".to_string(), Value::Table(Map::new()));
        }
        let dev = profile.get_mut("dev").unwrap().as_table_mut().unwrap();
        dev.insert("opt-level".to_string(), Value::Integer(opt_level as i64));
        dev.insert("debug".to_string(), Value::Integer(debug_level as i64));
        dev.insert("codegen-units".to_string(), Value::Integer(codegen_units as i64));
        dev.insert("lto".to_string(), Value::Boolean(false));
        if !config.as_table().unwrap().contains_key("env") {
            config
                .as_table_mut()
                .unwrap()
                .insert("env".to_string(), Value::Table(Map::new()));
        }
        let env = config.get_mut("env").unwrap().as_table_mut().unwrap();
        env.insert(
            "CARGO_INCREMENTAL".to_string(),
            Value::String(if incremental { "1" } else { "0" }.to_string()),
        );
        env.insert("CARGO_BUILD_JOBS".to_string(), Value::String(jobs.to_string()));
        Ok(())
    }
    fn show_optimization_summary(&self, config: &Value) -> Result<()> {
        println!("\n🚀 Build Optimization Summary:");
        println!("{}", "═".repeat(50).blue());
        if let Some(build) = config.get("build") {
            if let Some(jobs) = build.get("jobs") {
                println!("📊 Parallel Jobs: {}", jobs);
            }
            if let Some(incremental) = build.get("incremental") {
                println!("🔄 Incremental: {}", incremental);
            }
        }
        if let Some(profile) = config.get("profile") {
            if let Some(dev) = profile.get("dev") {
                if let Some(opt_level) = dev.get("opt-level") {
                    println!("⚡ Optimization Level: {}", opt_level);
                }
                if let Some(debug) = dev.get("debug") {
                    println!("🐛 Debug Level: {}", debug);
                }
                if let Some(codegen_units) = dev.get("codegen-units") {
                    println!("🏗️  Codegen Units: {}", codegen_units);
                }
                if let Some(lto) = dev.get("lto") {
                    println!("🔗 Link-Time Optimization: {}", lto);
                }
            }
        }
        if let Some(env) = config.get("env") {
            println!("\n🌍 Environment Variables:");
            for (key, value) in env.as_table().unwrap() {
                println!("  {} = {}", key, value);
            }
        }
        println!("{}", "═".repeat(50).blue());
        println!("💡 Run 'cargo build' to see the speed improvements!");
        Ok(())
    }
    pub fn restore_backup(&self) -> Result<()> {
        let cargo_toml_path = self.project_root.join("Cargo.toml");
        let backup_path = cargo_toml_path.with_extension("toml.backup");
        if !backup_path.exists() {
            return Err(anyhow::anyhow!("No backup found to restore"));
        }
        fs::copy(&backup_path, &cargo_toml_path)?;
        println!("✅ Restored Cargo.toml from backup");
        Ok(())
    }
    pub fn show_status(&self) -> Result<()> {
        let cargo_toml_path = self.project_root.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Err(anyhow::anyhow!("Cargo.toml not found"));
        }
        let content = fs::read_to_string(&cargo_toml_path)?;
        let config: Value = toml::from_str(&content)?;
        println!("🔍 Current Build Optimization Status:");
        println!("{}", "═".repeat(50).blue());
        if let Some(build) = config.get("build") {
            println!("📊 Build Configuration:");
            for (key, value) in build.as_table().unwrap() {
                println!("  {}: {}", key, value);
            }
        } else {
            println!("📊 Build Configuration: Not configured");
        }
        if let Some(profile) = config.get("profile") {
            if let Some(dev) = profile.get("dev") {
                println!("\n⚡ Dev Profile:");
                for (key, value) in dev.as_table().unwrap() {
                    println!("  {}: {}", key, value);
                }
            }
        }
        if let Some(env) = config.get("env") {
            println!("\n🌍 Environment Variables:");
            for (key, value) in env.as_table().unwrap() {
                println!("  {}: {}", key, value);
            }
        }
        println!("{}", "═".repeat(50).blue());
        Ok(())
    }
    pub fn get_optimal_jobs(&self) -> u32 {
        std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(4)
    }
    pub fn show_recommendations(&self) -> Result<()> {
        let cpu_count = self.get_optimal_jobs();
        println!("💡 Build Optimization Recommendations:");
        println!("{}", "═".repeat(50).blue());
        println!("🖥️  CPU Cores: {}", cpu_count);
        println!("📊 Recommended Jobs: {}", cpu_count);
        println!();
        println!("🚀 Aggressive Profile:");
        println!("  - Parallel jobs: {}", cpu_count);
        println!("  - Incremental: true");
        println!("  - Opt level: 1 (basic optimizations)");
        println!("  - Codegen units: 256 (maximum parallelism)");
        println!("  - Debug: 1 (reduced debug info)");
        println!();
        println!("⚖️  Balanced Profile:");
        println!("  - Parallel jobs: {}", cpu_count / 2);
        println!("  - Incremental: true");
        println!("  - Opt level: 1 (basic optimizations)");
        println!("  - Codegen units: 128 (moderate parallelism)");
        println!("  - Debug: 1 (reduced debug info)");
        println!();
        println!("🛡️  Conservative Profile:");
        println!("  - Parallel jobs: 2");
        println!("  - Incremental: true");
        println!("  - Opt level: 0 (no optimizations)");
        println!("  - Codegen units: 64 (minimal parallelism)");
        println!("  - Debug: 2 (full debug info)");
        println!();
        println!("💡 Use 'cm optimize aggressive' for maximum speed");
        println!("💡 Use 'cm optimize balanced' for good speed/stability");
        println!("💡 Use 'cm optimize conservative' for maximum stability");
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub enum OptimizationProfile {
    Aggressive,
    Balanced,
    Conservative,
    Custom {
        jobs: u32,
        incremental: bool,
        opt_level: u32,
        debug_level: u32,
        codegen_units: u32,
    },
}
impl OptimizationProfile {
    pub fn to_string(&self) -> &'static str {
        match self {
            OptimizationProfile::Aggressive => "Aggressive",
            OptimizationProfile::Balanced => "Balanced",
            OptimizationProfile::Conservative => "Conservative",
            OptimizationProfile::Custom { .. } => "Custom",
        }
    }
}
/// 🦀 **Crew Function #8**
/// Ship crew that validates command operations
pub fn check_crew_operations(command: &str) -> Result<bool> {
    println!(
        "👥 Crew checking operations for command '{}' - all hands accounted for!",
        command.cyan()
    );
    let license_manager = captain::license::LicenseManager::new()?;
    match license_manager.enforce_license(command) {
        Ok(_) => {
            println!(
                "✅ Crew reports: Command '{}' ready for operations!", command.green()
            );
            println!("   👥 All crew stations manned - ready to execute!");
            Ok(true)
        }
        Err(e) => {
            if e.to_string().contains("limit") {
                println!("⚠️  Crew warning: Operation quota exceeded!");
                println!("   👥 Resupply crew at: https://cargo.do/checkout");
                println!("   👥 Upgrade for unlimited crew operations");
            } else if e.to_string().contains("License not found") {
                println!("❌ Crew emergency: No operation authorization!");
                println!("   👥 Get clearance with 'cm register <key>'");
            } else {
                println!(
                    "❌ Crew distress: Operations check failed: {}", e.to_string().red()
                );
                println!("   👥 Secure all stations - prepare for inspection!");
            }
            Ok(false)
        }
    }
}