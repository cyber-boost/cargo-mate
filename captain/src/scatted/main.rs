use anyhow::Result;
use clap::{Parser, Subcommand};
use scat::{*, protection::ProtectionLevel};
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(
    name = "scat",
    about = "obfuSCATe - Advanced binary protector that makes your binaries look like 💩",
    version,
    author
)]
struct Cli {
    #[arg(value_name = "INPUT")]
    input: PathBuf,
    #[arg(value_name = "OUTPUT")]
    output: PathBuf,
    #[arg(short, long, default_value = "standard")]
    level: ProtectionLevel,
    #[command(subcommand)]
    mode: Mode,
}
#[derive(Subcommand, Debug)]
enum Mode {
    Single { #[arg(short, long)] key: Option<String> },
    Double {
        #[arg(short, long)]
        manifest: Option<PathBuf>,
        #[arg(short, long)]
        key: Option<String>,
    },
    SelfContained {
        #[arg(short, long)]
        obfuscate: bool,
        #[arg(short, long)]
        fetch_key: bool,
        #[arg(short, long)]
        key: Option<String>,
    },
    Ultra {
        #[arg(short, long)]
        report: Option<PathBuf>,
        #[arg(short, long)]
        hardware_lock: bool,
        #[arg(short, long)]
        metamorphic: bool,
    },
}
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    println!(
        r#"
    ┌─┐┌─┐┌─┐┌┬┐
    └─┐│  ├─┤ │ 
    └─┘└─┘┴ ┴ ┴ 
    obfuSCATe v2.0 - Making binaries look like 💩
    "#
    );
    let mut protector = protection::BinaryProtector::new(cli.level);
    match cli.mode {
        Mode::Single { key } => {
            protector.protect_single(&cli.input, &cli.output, key).await?;
        }
        Mode::Double { manifest, key } => {
            protector.protect_double(&cli.input, &cli.output, manifest, key).await?;
        }
        Mode::SelfContained { obfuscate, fetch_key, key } => {
            protector
                .protect_self_contained(
                    &cli.input,
                    &cli.output,
                    key,
                    obfuscate,
                    fetch_key,
                )
                .await?;
        }
        Mode::Ultra { report, hardware_lock, metamorphic } => {
            protector
                .protect_ultra(
                    &cli.input,
                    &cli.output,
                    report,
                    hardware_lock,
                    metamorphic,
                )
                .await?;
        }
    }
    println!("\n✨ Protection complete! Your binary now looks like digital sewage.");
    Ok(())
}