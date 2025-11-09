use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
#[derive(Parser, Debug)]
#[command(name = "cm")]
#[command(about = "Cargo Mate", long_about = None)]
#[command(version, author)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}
#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    Init,
    Journey { #[command(subcommand)] action: JourneyAction },
    Anchor { #[command(subcommand)] action: AnchorAction },
    Log { #[command(subcommand)] action: LogAction },
    Tide { #[command(subcommand)] action: TideAction },
    Map { #[command(subcommand)] action: MapAction },
    Mutiny { #[command(subcommand)] action: MutinyAction },
    Config { #[command(subcommand)] action: crate::ConfigAction },
    Version { #[command(subcommand)] action: VersionAction },
    View { #[command(subcommand)] action: ViewAction },
    Test,
    Probe { #[command(subcommand)] action: crate::probe::ProbeAction },
    Optimize { #[command(subcommand)] action: OptimizeAction },
    Checklist { #[command(subcommand)] action: ChecklistAction },
    History {
        #[arg(default_value = "summary")]
        kind: String,
        #[arg(default_value = "50")]
        limit: usize,
    },
    Scrub { #[command(subcommand)] action: ScrubAction },
    Sweep { #[command(subcommand)] action: crate::sweeping::SweepCommands },
    Install,
    Activate,
    Exec {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cargo_args: Vec<String>,
    },
    Register {
        license_key: Option<String>,
        #[arg(long)]
        status: bool,
        #[arg(long)]
        remaining: bool,
    },
    Idea { idea: String },
    Wtf { #[command(subcommand)] action: crate::captain::wtf::WtfAction },
    User,
    Debug,
    Strip(crate::strip::StripArgs),
    Scat { #[command(subcommand)] command: crate::scat::ScatCommand },
    Tool { #[command(subcommand)] action: ToolAction },
    Ddr { #[command(subcommand)] action: DockDockRustCommands },
    Deps {
        #[arg(long)]
        path: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Liberate {
        #[arg(short = 't', long, default_value = ".")]
        target: PathBuf,
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
    },
    Tree {
        #[command(subcommand)]
        action: Option<TreeAction>,
        #[arg(short = 't', long, default_value = ".")]
        target: PathBuf,
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        #[arg(long)]
        no_folders: bool,
        #[arg(long)]
        no_files: bool,
        #[arg(long)]
        folder_size: bool,
        #[arg(long)]
        file_size: bool,
        #[arg(long)]
        line_count: bool,
        #[arg(long)]
        dates: bool,
        #[arg(long, default_value = "readme")]
        style: String,
        #[arg(long)]
        yolo: bool,
    },
    Stub {
        #[command(subcommand)]
        action: Option<StubAction>,
        #[arg(short = 't', long, default_value = ".")]
        target: PathBuf,
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        #[arg(long)]
        ext: Option<String>,
        #[arg(long)]
        custom: Option<String>,
        #[arg(long)]
        find: Option<String>,
        #[arg(long)]
        skip: Option<String>,
    },
    Bin {
        #[command(subcommand)]
        action: Option<BinAction>,
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short = 'n', long)]
        name: Option<String>,
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
        #[arg(long, default_value = "10")]
        timeout_seconds: u64,
        #[arg(long)]
        max_depth: Option<usize>,
    },
}
impl Commands {
    pub fn name(&self) -> &'static str {
        match self {
            Commands::Init => "init",
            Commands::Journey { .. } => "journey",
            Commands::Anchor { .. } => "anchor",
            Commands::Log { .. } => "log",
            Commands::Tide { .. } => "tide",
            Commands::Map { .. } => "map",
            Commands::Mutiny { .. } => "mutiny",
            Commands::Config { .. } => "config",
            Commands::Version { .. } => "version",
            Commands::View { .. } => "view",
            Commands::Test => "test",
            Commands::Probe { .. } => "probe",
            Commands::Optimize { .. } => "optimize",
            Commands::Checklist { .. } => "checklist",
            Commands::History { .. } => "history",
            Commands::Scrub { .. } => "scrub",
            Commands::Install => "install",
            Commands::Activate => "activate",
            Commands::Exec { .. } => "exec",
            Commands::Register { .. } => "register",
            Commands::Idea { .. } => "idea",
            Commands::Wtf { .. } => "wtf",
            Commands::User => "user",
            Commands::Debug => "debug",
            Commands::Strip(_) => "strip",
            Commands::Scat { .. } => "scat",
            Commands::Sweep { action: _ } => "sweep",
            Commands::Tool { .. } => "tool",
            Commands::Ddr { .. } => "ddr",
            Commands::Deps { .. } => "deps",
            Commands::Liberate { .. } => "liberate",
            Commands::Tree { .. } => "tree",
            Commands::Stub { .. } => "stub",
            Commands::Bin { .. } => "bin",
        }
    }
}
#[derive(Subcommand, Debug, Clone)]
pub enum JourneyAction {
    Record { name: String },
    Play { name: String, #[arg(long)] dry_run: bool },
    List,
    Export { name: String, output: PathBuf },
    Import { path: PathBuf },
    Publish { name: String, #[arg(long)] tags: Vec<String> },
    Download { gist_id: String },
    Search { query: String },
    Published,
}
#[derive(Subcommand, Debug, Clone)]
pub enum AnchorAction {
    Save { name: String, #[arg(long)] message: Option<String> },
    Restore { name: String },
    List,
    Show { name: String },
    Diff { name: String },
    Auto { name: String, #[arg(long)] foreground: bool },
    Stop { name: String },
}
#[derive(Subcommand, Debug, Clone)]
pub enum LogAction {
    Add { message: String, #[arg(long)] tags: Vec<String> },
    Search { query: String },
    Timeline { #[arg(default_value = "7")] days: i64 },
    Export { path: PathBuf, #[arg(long, default_value = "markdown")] format: String },
    Analyze,
    Track { command: String },
}
#[derive(Subcommand, Debug, Clone)]
pub enum TideAction {
    Show,
    Analyze,
    Export { path: PathBuf },
}
#[derive(Subcommand, Debug, Clone)]
pub enum MapAction {
    Show,
    Analyze,
    Export { path: PathBuf },
    Path { from: String, to: String },
}
#[derive(Subcommand, Debug, Clone)]
pub enum MutinyAction {
    Activate { reason: String },
    Deactivate,
    AllowWarnings,
    SkipTests,
    Force,
    Yolo,
    Status,
}
#[derive(Subcommand, Debug, Clone)]
pub enum VersionAction {
    Init { version: Option<String> },
    Info,
    Increment { increment_type: IncrementType },
    Set { version: String },
    History,
    UpdateCargo,
    Config { #[command(subcommand)] action: VersionConfigAction },
}
#[derive(Subcommand, Debug, Clone)]
pub enum VersionConfigAction {
    Enable,
    Disable,
    Policy { #[arg(value_enum)] policy: IncrementType },
    Show,
}
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum IncrementType {
    Patch,
    Minor,
    Major,
}
#[derive(Subcommand, Debug, Clone)]
pub enum ViewAction {
    Errors,
    Artifacts,
    Scripts,
    History,
    Checklist,
    All,
    Latest,
    Open,
}
#[derive(Subcommand, Debug, Clone)]
pub enum OptimizeAction {
    Aggressive,
    Balanced,
    Conservative,
    Custom {
        #[arg(default_value = "4")]
        jobs: u32,
        #[arg(default_value = "true")]
        incremental: String,
        #[arg(default_value = "3")]
        opt_level: u32,
        #[arg(default_value = "0")]
        debug_level: u32,
        #[arg(default_value = "128")]
        codegen_units: u32,
    },
    Status,
    Recommendations,
    Restore,
}
#[derive(Subcommand, Debug, Clone)]
pub enum ChecklistAction {
    Show,
    List,
    Add { item: String },
    Done { items: String },
    Clear { #[arg(default_value = "done")] target: String },
}
#[derive(Subcommand, Debug, Clone)]
pub enum ScrubAction {
    Run {
        #[arg(long)]
        dry_run: bool,
        #[arg(short, long)]
        verbose: bool,
        #[arg(short, long, default_value = "/")]
        start: String,
        #[arg(short, long)]
        resume: Option<String>,
        #[arg(long, default_value = "1")]
        min_depth: usize,
        #[arg(long, default_value = "10")]
        max_depth: usize,
    },
    Help,
}
#[derive(Subcommand, Debug, Clone)]
pub enum ToolAction {
    List,
    Help { name: String },
    Run { name: String, #[arg(trailing_var_arg = true)] args: Vec<String> },
    #[command(external_subcommand)]
    Execute(Vec<String>),
}
#[derive(Subcommand, Debug, Clone)]
pub enum SweepAction {
    Scan {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        include_tests: bool,
        #[arg(long)]
        include_examples: bool,
    },
    Clean {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        force: bool,
    },
    Init,
    Help,
}
#[derive(Subcommand, Debug, Clone)]
pub enum DdrCommands {
    #[command(flatten)]
    DockDockRust(DockDockRustCommands),
}
#[derive(Subcommand, Debug, Clone)]
pub enum DockDockRustCommands {
    DockDockRust {
        /// Docker image to use
        #[arg(short, long, default_value = "rust:latest")]
        image: String,
        /// Path to Cargo.toml
        #[arg(short = 'c', long, default_value = "Cargo.toml")]
        cargo: PathBuf,
        /// Source directory
        #[arg(short = 's', long, default_value = "src")]
        src: PathBuf,
        /// Build targets (can specify multiple)
        #[arg(short = 't', long)]
        target: Vec<String>,
        /// Configuration file
        #[arg(short = 'f', long, default_value = "ddr.toml")]
        config: PathBuf,
        /// Maximum parallel jobs
        #[arg(short = 'j', long, default_value = "16")]
        jobs: usize,
        /// Generate config only
        #[arg(long)]
        gen_config: bool,
        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },
}
#[derive(Subcommand, Debug, Clone)]
pub enum TreeAction {
    History,
    Show { name: String },
    Find { query: String },
}
#[derive(Subcommand, Debug, Clone)]
pub enum StubAction {
    Find { pattern: Option<String> },
    Skip { patterns: String },
    History,
    Show { name: String },
    Delete { #[arg(long)] all: bool },
}
#[derive(Subcommand, Debug, Clone)]
pub enum BinAction {
    History,
    Show { name: String },
    Find { query: String },
    Delete { #[arg(long)] all: bool },
}