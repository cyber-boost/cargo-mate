use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(name = "sweep")]
#[command(about = "🧹 Sweep - Debug statement cleaner and code formatter")]
pub struct SweepCli {
    #[command(subcommand)]
    command: SweepCommands,
    #[arg(short, long)]
    verbose: bool,
}
#[derive(Subcommand, Debug, Clone)]
pub enum SweepCommands {
    Scan {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        include_tests: bool,
        #[arg(long)]
        include_examples: bool,
        #[arg(long)]
        export: Option<PathBuf>,
    },
    Sweep {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        interactive: bool,
        #[arg(long)]
        prompt: bool,
        #[arg(long)]
        keep_main: bool,
        #[arg(long)]
        keep_tests: bool,
        #[arg(long)]
        keep_examples: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        pristine: bool,
        #[arg(long)]
        format: bool,
        #[arg(long)]
        organize_imports: bool,
        #[arg(long)]
        add_docs: bool,
        #[arg(long)]
        fix_clippy: bool,
    },
    Convert {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        println_level: Option<String>,
        #[arg(long)]
        eprintln_level: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        add_dependency: bool,
    },
    Analyze {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "10")]
        top: usize,
    },
    Init { #[arg(long)] force: bool },
    Help,
}
pub fn run_sweep(command: SweepCommands, verbose: bool) -> Result<()> {
    match command {
        SweepCommands::Scan { path, include_tests, include_examples, export } => {
            println!("🔍 Scanning for print statements in: {}", path.display());
            let sweeper = crate::sweeping::src::away::Sweeper::new();
            let statements = sweeper
                .scan_directory(&path, include_tests, include_examples)?;
            if statements.is_empty() {
                println!("{}", "✨ Clean! No print statements found.".green());
                return Ok(());
            }
            println!("\n{}\n", "📋 Scan Results".bold().blue());
            println!("{}", "─".repeat(60).dimmed());
            for stmt in &statements {
                sweeper.display_statement(stmt);
            }
            println!("\n{}: {}", "Total".bold(), statements.len());
            if let Some(export_path) = export {
                crate::sweeping::src::away::export_to_json(&statements, &export_path)?;
                println!("📄 Results exported to: {}", export_path.display());
            }
        }
        SweepCommands::Sweep {
            path,
            dry_run,
            interactive,
            prompt,
            keep_main,
            keep_tests,
            keep_examples,
            backup,
            yes,
            pristine,
            format,
            organize_imports,
            add_docs,
            fix_clippy,
        } => {
            println!("🧹 Cleaning debug statements in: {}", path.display());
            let sweeper = crate::sweeping::src::away::Sweeper::new();
            let statements = sweeper.scan_directory(&path, !keep_tests, !keep_examples)?;
            if statements.is_empty() {
                println!("{}", "✨ Already clean! No print statements found.".green());
                return Ok(());
            }
            println!("Found {} print statements to process", statements.len());
            if dry_run {
                println!("{}", "🔍 DRY RUN - Showing what would be changed:".yellow());
            }
            let mut sweeper = crate::sweeping::src::away::Sweeper::new();
            sweeper.load_config(&std::path::PathBuf::from(".sweep.toml"))?;
            let options = crate::sweeping::src::away::SweepOptions {
                dry_run,
                interactive,
                prompt,
                keep_main,
                keep_tests,
                keep_examples,
                backup,
                yes,
            };
            sweeper
                .sweep_files(
                    statements,
                    &options,
                    &std::path::PathBuf::from(".sweep.toml"),
                )?;
        }
        SweepCommands::Convert {
            path,
            println_level,
            eprintln_level,
            dry_run,
            add_dependency,
        } => {
            println!(
                "🔄 Converting print statements to logging in: {}", path.display()
            );
            let sweeper = crate::sweeping::src::away::Sweeper::new();
            let statements = sweeper.scan_directory(&path, true, true)?;
            if statements.is_empty() {
                println!("{}", "✨ No print statements found to convert!".green());
                return Ok(());
            }
            let println_level = println_level
                .map(|s| {
                    match s.to_lowercase().as_str() {
                        "trace" => crate::sweeping::src::away::LogLevel::Trace,
                        "info" => crate::sweeping::src::away::LogLevel::Info,
                        "warn" => crate::sweeping::src::away::LogLevel::Warn,
                        "error" => crate::sweeping::src::away::LogLevel::Error,
                        _ => crate::sweeping::src::away::LogLevel::Debug,
                    }
                })
                .unwrap_or(crate::sweeping::src::away::LogLevel::Debug);
            let eprintln_level = eprintln_level
                .map(|s| {
                    match s.to_lowercase().as_str() {
                        "trace" => crate::sweeping::src::away::LogLevel::Trace,
                        "info" => crate::sweeping::src::away::LogLevel::Info,
                        "warn" => crate::sweeping::src::away::LogLevel::Warn,
                        "error" => crate::sweeping::src::away::LogLevel::Error,
                        _ => crate::sweeping::src::away::LogLevel::Error,
                    }
                })
                .unwrap_or(crate::sweeping::src::away::LogLevel::Error);
            let mut converted = 0;
            let mut files_modified = 0;
            for stmt in statements {
                if !dry_run {
                    if let Err(e) = crate::sweeping::src::away::convert_statement_in_file(
                        &stmt.file,
                        &stmt.content,
                        &stmt.content,
                    ) {
                        eprintln!("Error converting {}: {}", stmt.file.display(), e);
                        continue;
                    }
                    converted += 1;
                    files_modified += 1;
                } else {
                    println!(
                        "Would convert: {} at {}:{}", stmt.file.display(), stmt.line,
                        stmt.column
                    );
                    converted += 1;
                }
            }
            if dry_run {
                println!(
                    "{} Would convert {} statements across {} files", "🔍".yellow(),
                    converted, files_modified
                );
            } else {
                println!(
                    "{} Converted {} statements across {} files", "✅".green(),
                    converted, files_modified
                );
            }
        }
        SweepCommands::Analyze { path, top } => {
            println!("📊 Analyzing print statement patterns in: {}", path.display());
            let sweeper = crate::sweeping::src::away::Sweeper::new();
            let statements = sweeper.scan_directory(&path, true, true)?;
            if statements.is_empty() {
                println!(
                    "{}", "✨ Perfectly clean! No print statements found.".green()
                );
                return Ok(());
            }
            let report = sweeper.analyze_patterns(&statements, top);
            sweeper.display_report(&report);
        }
        SweepCommands::Init { force } => {
            println!("⚙️  Initializing sweep configuration...");
            let config_path = std::path::PathBuf::from(".sweep.toml");
            if config_path.exists() && !force {
                println!(
                    "{} Config file already exists. Use --force to overwrite.", "⚠️"
                    .yellow()
                );
                return Ok(());
            }
            crate::sweeping::src::away::create_default_config(&config_path)?;
            println!(
                "{} Configuration initialized at: {}", "✅".green(), config_path
                .display()
            );
        }
        SweepCommands::Help => {
            println!("🧹 Sweep - Debug statement cleaner and code formatter");
            println!("\nCommands:");
            println!("  scan     - Scan for print statements");
            println!("  sweep    - Clean debug statements");
            println!("  convert  - Convert to logging");
            println!("  analyze  - Analyze patterns");
            println!("  init     - Initialize configuration");
            println!("  help     - Show this help message");
            return Ok(());
        }
    }
    Ok(())
}
pub mod src;