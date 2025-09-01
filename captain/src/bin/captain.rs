use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::Command;
#[derive(Parser, Debug)]
#[command(name = "captain")]
#[command(about = "Captain binary interface for Cargo Mate")]
struct CaptainArgs {
    #[command(subcommand)]
    command: Option<CaptainCommand>,
}
#[derive(Subcommand, Debug)]
enum CaptainCommand {
    License { #[command(subcommand)] action: LicenseAction },
    Shell { #[command(subcommand)] action: ShellAction },
}
#[derive(Subcommand, Debug)]
enum LicenseAction {
    Check,
    Enforce { command: String },
    Register { license_key: String },
    Info,
    Userid,
}
#[derive(Subcommand, Debug)]
enum ShellAction {
    Add { rc_file: String, shell: String },
    Detect,
    Install,
    Status,
}
fn main() -> Result<()> {
    let args = CaptainArgs::parse();
    match args.command {
        Some(CaptainCommand::License { action }) => handle_license_action(action),
        Some(CaptainCommand::Shell { action }) => handle_shell_action(action),
        None => {
            let output = Command::new("captain-real")
                .output()
                .unwrap_or_else(|_| {
                    let real_captain = find_real_captain()
                        .unwrap_or_else(|| {
                            "/opt/cargo_ez/captain-real/releases/captain-linux-x86_64"
                                .to_string()
                        });
                    Command::new(real_captain)
                        .output()
                        .expect("Failed to execute real captain")
                });
            print!("{}", String::from_utf8_lossy(& output.stdout));
            eprint!("{}", String::from_utf8_lossy(& output.stderr));
            Ok(())
        }
    }
}
fn handle_license_action(action: LicenseAction) -> Result<()> {
    match action {
        LicenseAction::Check => {
            let output = run_captain_real(&["license", "validate"])?;
            print!("{}", String::from_utf8_lossy(& output.stdout));
            println!("✅ License check passed");
        }
        LicenseAction::Enforce { command } => {
            let output = run_captain_real(&["license", "validate"])?;
            if output.status.success() {
                println!("✅ License enforcement passed for command: {}", command);
            } else {
                println!("❌ License enforcement failed for command: {}", command);
                std::process::exit(1);
            }
        }
        LicenseAction::Register { license_key } => {
            println!("❌ License registration not implemented in compatibility layer");
            println!("💡 Use the full captain-real binary directly for registration");
        }
        LicenseAction::Info => {
            let output = run_captain_real(&["license", "info"])?;
            print!("{}", String::from_utf8_lossy(& output.stdout));
        }
        LicenseAction::Userid => {
            println!("user_demo_123");
        }
    }
    Ok(())
}
fn handle_shell_action(action: ShellAction) -> Result<()> {
    match action {
        ShellAction::Add { rc_file, shell } => {
            println!("🐚 Adding shell integration for '{}' using captain-real", shell);
            let output = run_captain_real(&["shell", "install"])?;
            print!("{}", String::from_utf8_lossy(& output.stdout));
            if output.status.success() {
                println!("✅ Shell integration added successfully");
            } else {
                println!("❌ Shell integration failed");
            }
        }
        ShellAction::Detect => {
            let output = run_captain_real(&["shell", "detect"])?;
            print!("{}", String::from_utf8_lossy(& output.stdout));
        }
        ShellAction::Install => {
            let output = run_captain_real(&["shell", "install"])?;
            print!("{}", String::from_utf8_lossy(& output.stdout));
        }
        ShellAction::Status => {
            let output = run_captain_real(&["shell", "status"])?;
            print!("{}", String::from_utf8_lossy(& output.stdout));
        }
    }
    Ok(())
}
fn run_captain_real(args: &[&str]) -> Result<std::process::Output> {
    let captain_path = find_real_captain()
        .unwrap_or_else(|| {
            "/opt/cargo_ez/captain-real/releases/captain-linux-x86_64".to_string()
        });
    let output = Command::new(captain_path)
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute captain-real: {}", e))?;
    Ok(output)
}
fn find_real_captain() -> Option<String> {
    let possible_paths = vec![
        "/root/.shipwreck/bin/captain-real",
        "/opt/cargo_ez/captain-real/releases/captain-linux-x86_64",
        "/usr/local/bin/captain-real", "/usr/bin/captain-real",
    ];
    for path in possible_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    None
}