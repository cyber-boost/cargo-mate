pub mod encryption;
pub mod embedder;
use anyhow::Result;
use clap::{Parser, Subcommand};
#[derive(Parser, Debug)]
#[command(name = "sweep")]
#[command(
    about = "🧹 Sweep away println! and eprintln! debug statements from Rust code"
)]
pub struct SweepCli {
    #[command(subcommand)]
    command: SweepCommands,
    #[arg(short, long)]
    verbose: bool,
}
#[derive(Subcommand, Debug)]
enum SweepCommands {
    Scan { #[arg(default_value = ".")] path: String, #[arg(long)] include_tests: bool },
    Sweep {
        #[arg(default_value = ".")]
        path: String,
        #[arg(short, long)]
        dry_run: bool,
    },
    Convert { #[arg(default_value = ".")] path: String },
}
pub fn run_sweep() -> Result<()> {
    let cli = SweepCli::parse();
    let args = vec!["sweep"];
    match cli.command {
        SweepCommands::Scan { path, include_tests } => {
            let mut cmd_args = vec!["scan", & path];
            if include_tests {
                cmd_args.push("--include-tests");
            }
            let output = embedder::execute_sweep_binary(&cmd_args)?;
            print_output(&output);
        }
        SweepCommands::Sweep { path, dry_run } => {
            let mut cmd_args = vec!["sweep", & path];
            if dry_run {
                cmd_args.push("--dry-run");
            }
            let output = embedder::execute_sweep_binary(&cmd_args)?;
            print_output(&output);
        }
        SweepCommands::Convert { path } => {
            let cmd_args = vec!["convert", & path];
            let output = embedder::execute_sweep_binary(&cmd_args)?;
            print_output(&output);
        }
    }
    Ok(())
}
fn print_output(output: &std::process::Output) {
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(& output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(& output.stderr));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sweep_cli_parsing() {
        let args = vec!["sweep", "scan", "."];
        let cli = SweepCli::try_parse_from(args).unwrap();
        match cli.command {
            SweepCommands::Scan { path, include_tests } => {
                assert_eq!(path, ".");
                assert!(! include_tests);
            }
            _ => panic!("Expected scan command"),
        }
    }
}