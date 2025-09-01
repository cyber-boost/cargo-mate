use anyhow::{Context, Result};
use clap::Subcommand;
use colored::*;
use std::process::Command;
#[derive(Subcommand, Debug, Clone)]
pub enum WtfAction {
    Ask { input: String, #[arg(long)] file: bool },
    #[command(hide = true)]
    Direct { input: String, #[arg(long)] file: bool },
    Er { #[arg(default_value = "10")] count: usize },
    Ollama { #[command(subcommand)] command: OllamaCommand },
    List { #[arg(default_value = "10")] limit: usize },
    Show { id: String },
    History { #[arg(default_value = "10")] limit: usize },
    Checklist { #[arg(default_value = "10")] limit: usize },
}
#[derive(Subcommand, Debug, Clone)]
pub enum OllamaCommand {
    Enable { #[arg(default_value = "llama2")] model: String },
    Disable,
    Status,
    Models,
}
pub fn display_api_failure_art() {
    println!();
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⠀⠀⠀⠀⢻⡍⠛⠶⣤⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⠀⠀⠀⢀⡾⠁⠀⠀⠀⢙⣦⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⠀⠀⠀⣾⣀⣀⣀⠴⠚⠁⠈⢷⡀⠀⠀⠀⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⠀⣴⠟⠉⠀⠀⠀⠀⠀⠀⢀⡸⢧⣄⡀⠀⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⠀⣾⠁⠀⢀⠀⠀⠀⢀⣠⠔⠋⢀⡀⠈⢻⡆⠀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⠀⠀⣀⣼⣦⠋⣉⡉⢲⡚⠉⠀⢠⠞⣉⣉⠳⡼⢧⣀⠀⠀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⢀⡾⠋⠀⡇⢰⣿⣿⠀⣧⠀⠀⡏⢸⣿⣿⡆⢹⠀⠉⢷⡀⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⠀⠀⢸⡇⠀⠀⢧⠈⠿⠟⢠⣇⠤⠖⢳⡈⠻⠟⢁⡞⠀⠀⢸⡇⠀⠀"
        .red()
    );
    println!(
        "{}",
        "⢀⣠⠶⠓⠒⠒⠒⠓⢒⡚⠁⠀⠀⠀⠀⠙⢒⡒⠋⠀⠀⣠⠿⢦⣄⠀"
        .red()
    );
    println!(
        "{}",
        "⣾⠁⠀⠀⠀⠀⠀⠀⠫⣌⠉⠉⠉⠉⠉⠉⣩⠟⢀⡤⠚⠁⠀⠀⠙⣆"
        .red()
    );
    println!(
        "{}",
        "⣧⠀⠀⠀⠀⠀⠀⠀⠀⠈⠙⠒⠒⣒⡲⠭⠒⠊⠁⠀⠀⠀⠀⠀⢀⡿"
        .red()
    );
    println!(
        "{}",
        "⠹⢦⣄⣀⣀⣠⣤⠤⠴⠶⠶⣬⣭⣄⣀⣀⣀⣀⣀⣀⣀⣤⡤⠶⠋⠀"
        .red()
    );
    println!();
    println!("{}", "Well, this is awkward!".bright_blue().bold());
    println!("{}", "Hopefully someone is fixing this.".cyan());
    println!("{}", "Want to help? Please contact us at mate@cargo.do".cyan());
    println!();
}
pub fn handle_wtf_action(action: WtfAction) -> Result<()> {
    println!("🚀 {}", "CargoMate AI - Pro Feature".bright_blue().bold());
    println!("   Delegating to advanced captain binary...");
    let args: Vec<String> = match action {
        WtfAction::Ask { input, file } => {
            if file {
                vec!["wtf".to_string(), "ask".to_string(), input, "--file".to_string()]
            } else {
                vec!["wtf".to_string(), "ask".to_string(), input]
            }
        }
        WtfAction::Direct { input, file } => {
            if file {
                vec![
                    "wtf".to_string(), "direct".to_string(), input, "--file".to_string()
                ]
            } else {
                vec!["wtf".to_string(), "direct".to_string(), input]
            }
        }
        WtfAction::Er { count } => {
            vec!["wtf".to_string(), "er".to_string(), count.to_string()]
        }
        WtfAction::Ollama { command } => {
            match command {
                OllamaCommand::Enable { model } => {
                    vec![
                        "wtf".to_string(), "ollama".to_string(), "enable".to_string(),
                        model
                    ]
                }
                OllamaCommand::Disable => {
                    vec!["wtf".to_string(), "ollama".to_string(), "disable".to_string()]
                }
                OllamaCommand::Status => {
                    vec!["wtf".to_string(), "ollama".to_string(), "status".to_string()]
                }
                OllamaCommand::Models => {
                    vec!["wtf".to_string(), "ollama".to_string(), "models".to_string()]
                }
            }
        }
        WtfAction::List { limit } => {
            vec!["wtf".to_string(), "list".to_string(), limit.to_string()]
        }
        WtfAction::Show { id } => vec!["wtf".to_string(), "show".to_string(), id],
        WtfAction::History { limit } => {
            vec!["wtf".to_string(), "history".to_string(), limit.to_string()]
        }
        WtfAction::Checklist { limit } => {
            vec!["wtf".to_string(), "checklist".to_string(), limit.to_string()]
        }
    };
    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    delegate_to_captain_binary(&str_args)
}
fn delegate_to_captain_binary(args: &[&str]) -> Result<()> {
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
                    println!("   Or visit: https://cargo.do/pro");
                    return Ok(());
                }
            }
        }
    };
    let output = Command::new(&captain_path)
        .args(args)
        .output()
        .context("Failed to execute captain binary")?;
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
        display_api_failure_art();
    }
    Ok(())
}