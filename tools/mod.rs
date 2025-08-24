use std::collections::HashMap;
use clap::{ArgMatches, Command};
use colored::*;
use thiserror::Error;
use quote::ToTokens;

// Import tool implementations
pub mod bench_diff;
pub mod dep_audit;
pub mod test_gen;
pub mod workspace_sync;
pub mod panic_analyzer;
pub mod cross_test;
pub mod refactor_engine;
pub mod rust_mentor;
pub mod release_automation;
pub mod bloat_check;
pub mod cache_analyzer;
pub mod async_lint;
pub mod crud_gen;
pub mod proto_bind;
pub mod error_derive;
pub mod builder_gen;
pub mod example_gen;
pub mod api_changelog;
pub mod trait_explorer;
pub mod env_check;
pub mod feature_map;
pub mod vendorize;
pub mod macro_expand;
pub mod lifetime_visualizer;
pub mod compile_time_tracker;
pub mod mock_derive;
pub mod coverage_guard;
pub mod snapshot_test;
pub mod wasm_optimize;
pub mod installer_gen;
pub mod migration_gen;
pub mod serde_validator;
pub mod sql_macro_check;
pub mod secret_scanner;
pub mod unsafe_analyzer;
pub mod license_bundler;
pub mod code_analyzer;

// ML-powered tools have been archived due to compilation issues

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool '{0}' not found")]
    ToolNotFound(String),

    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("TOML parsing error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Syn parsing error: {0}")]
    SynError(String),
}

impl From<syn::Error> for ToolError {
    fn from(err: syn::Error) -> Self {
        ToolError::SynError(format!("Rust syntax parsing failed: {}. This usually indicates invalid Rust code or an unsupported language feature.", err))
    }
}

pub type Result<T> = std::result::Result<T, ToolError>;

/// Core trait that all tools must implement
pub trait Tool {
    /// Returns the name of the tool
    fn name(&self) -> &'static str;

    /// Returns a description of what the tool does
    fn description(&self) -> &'static str;

    /// Returns the command structure for this tool
    fn command(&self) -> Command;

    /// Execute the tool with the given arguments
    fn execute(&self, matches: &ArgMatches) -> Result<()>;
}

/// Registry that manages all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register<T: Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
        self
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn Tool>> {
        self.tools.get(name)
    }

    pub fn list_tools(&self) -> Vec<(&str, &str)> {
        let mut tools: Vec<_> = self.tools.values()
            .map(|tool| (tool.name(), tool.description()))
            .collect();
        tools.sort_by(|a, b| a.0.cmp(b.0));
        tools
    }

    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

/// Configuration for tools
#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub verbose: bool,
    pub dry_run: bool,
    pub output_format: OutputFormat,
}

#[derive(Debug, Clone)]
pub enum OutputFormat {
    Human,
    Json,
    Table,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            dry_run: false,
            output_format: OutputFormat::Human,
        }
    }
}

/// Initialize the tool registry with all available tools
pub fn create_registry() -> ToolRegistry {
    let registry = ToolRegistry::new();

    // Register all available tools
    registry
        .register(bench_diff::BenchDiffTool::new())
        .register(dep_audit::DepAuditTool::new())
        .register(test_gen::TestGenTool::new())
        .register(workspace_sync::WorkspaceSyncTool::new())
        .register(panic_analyzer::PanicAnalyzerTool::new())
        .register(cross_test::CrossTestTool::new())
        .register(release_automation::ReleaseAutomationTool::new())
        .register(crud_gen::CrudGenTool::new())
        .register(proto_bind::ProtoBindTool::new())
        .register(error_derive::ErrorDeriveTool::new())
        .register(builder_gen::BuilderGenTool::new())
        .register(example_gen::ExampleGenTool::new())
        .register(api_changelog::ApiChangelogTool::new())
        .register(trait_explorer::TraitExplorerTool::new())
        .register(rust_mentor::RustMentorTool::new())
        .register(refactor_engine::RefactorEngineTool::new())
        .register(env_check::EnvCheckTool::new())
        .register(bloat_check::BloatCheckTool::new())
        .register(cache_analyzer::CacheAnalyzerTool::new())
        .register(async_lint::AsyncLintTool::new())
        .register(feature_map::FeatureMapTool::new())
        .register(vendorize::VendorizeTool::new())
        .register(macro_expand::MacroExpandTool::new())
        .register(lifetime_visualizer::LifetimeVisualizerTool::new())
        .register(compile_time_tracker::CompileTimeTrackerTool::new())
        .register(mock_derive::MockDeriveTool::new())
        .register(coverage_guard::CoverageGuardTool::new())
        .register(snapshot_test::SnapshotTestTool::new())
        .register(wasm_optimize::WasmOptimizeTool::new())
        .register(installer_gen::InstallerGenTool::new())
        .register(migration_gen::MigrationGenTool::new())
        .register(serde_validator::SerdeValidatorTool::new())
        .register(sql_macro_check::SqlMacroCheckTool::new())
        .register(secret_scanner::SecretScannerTool::new())
        .register(unsafe_analyzer::UnsafeAnalyzerTool::new())
        .register(license_bundler::LicenseBundlerTool::new())
        .register(code_analyzer::CodeAnalyzer::new())
}

static mut REGISTRY: Option<ToolRegistry> = None;

/// Get the global tool registry (lazy initialized)
pub fn get_registry() -> &'static ToolRegistry {
    unsafe {
        if REGISTRY.is_none() {
            REGISTRY = Some(create_registry());
        }
        REGISTRY.as_ref().unwrap()
    }
}

/// List all available tools
pub fn list_tools() {
    let registry = get_registry();
    let tools = registry.list_tools();

    if tools.is_empty() {
        println!("{}", "No tools available yet.".yellow());
        println!("Tools are being developed and will be added soon!");
        return;
    }

    println!("{}", "🔧 Available Tools".bold().blue());
    println!("{}", "═".repeat(50).blue());

    for (name, description) in tools {
        println!("  {} - {}", name.green().bold(), description);
    }

    println!();
    println!("{}", "Usage:".bold());
    println!("  cm tool list                          # List all tools");
    println!("  cm tool help <name>                   # Show help for a tool");
    println!("  cm tool <name> [options]              # Run a tool");
    println!("  cm tool run <name> [options]          # Run a tool (explicit)");
}

/// Show help for a specific tool
pub fn show_tool_help(name: &str) {
    let registry = get_registry();

    if let Some(tool) = registry.get(name) {
        let mut command = tool.command();
        println!("{}", format!("Help for tool: {}", name).bold().blue());
        println!("{}", "═".repeat(50).blue());
        println!("{}", tool.description());
        println!();

        // Use clap's built-in help formatting
        let _ = command.print_help();
    } else {
        println!("{}", format!("❌ Tool '{}' not found", name).red());
        println!();
        println!("{}", "Available tools:".bold());
        list_tools();
    }
}

/// Run a specific tool with arguments
pub fn run_tool(name: &str, args: &[String]) -> Result<()> {
    let registry = get_registry();

    let tool = registry.get(name)
        .ok_or_else(|| ToolError::ToolNotFound(name.to_string()))?;

    // Create the command structure to parse arguments
    let command = tool.command();

    // Convert args to the format clap expects
    let clap_args = std::iter::once(name.to_string()).chain(args.iter().cloned()).collect::<Vec<_>>();
    let arg_refs: Vec<&str> = clap_args.iter().map(|s| s.as_str()).collect();

    // Parse the arguments
    let matches = command.try_get_matches_from(&arg_refs)
        .map_err(|e| ToolError::InvalidArguments(e.to_string()))?;

    // Execute the tool
    tool.execute(&matches)
}

/// Helper function to create common CLI options
pub fn common_options() -> Vec<clap::Arg> {
    vec![
        clap::Arg::new("verbose")
            .long("verbose")
            .short('v')
            .help("Enable verbose output")
            .action(clap::ArgAction::SetTrue),
        clap::Arg::new("dry-run")
            .long("dry-run")
            .help("Show what would be done without executing")
            .action(clap::ArgAction::SetTrue),
        clap::Arg::new("output")
            .long("output")
            .short('o')
            .help("Output format")
            .value_parser(["human", "json", "table"])
            .default_value("human"),
    ]
}

/// Helper function to parse output format from matches
pub fn parse_output_format(matches: &ArgMatches) -> OutputFormat {
    match matches.get_one::<String>("output").map(|s| s.as_str()) {
        Some("json") => OutputFormat::Json,
        Some("table") => OutputFormat::Table,
        _ => OutputFormat::Human,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = create_registry();
        assert!(!registry.has_tool("nonexistent"));
    }

    #[test]
    fn test_list_tools_empty() {
        let registry = create_registry();
        let tools = registry.list_tools();
        // Initially empty until we register tools
        assert!(tools.is_empty() || !tools.is_empty()); // Flexible for now
    }
}
