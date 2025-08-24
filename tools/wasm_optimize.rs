use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::fs;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct WasmOptimizeTool;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OptimizationReport {
    original_size: u64,
    optimized_size: u64,
    reduction_percentage: f64,
    steps_completed: Vec<String>,
    build_time: f64,
    tools_used: Vec<String>,
    recommendations: Vec<String>,
    timestamp: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WasmAnalysis {
    file_size: u64,
    function_count: usize,
    export_count: usize,
    import_count: usize,
    has_debug_info: bool,
    has_names_section: bool,
    optimization_level: String,
}

impl WasmOptimizeTool {
    pub fn new() -> Self {
        Self
    }

    fn check_wasm_tools(&self) -> Result<Vec<String>> {
        let tools = vec![
            ("wasm-pack", "WebAssembly build tool"),
            ("wasm-opt", "Binaryen optimizer"),
            ("wasm-strip", "Debug info stripper"),
            ("twiggy", "WASM size analyzer"),
            ("cargo-wasm", "Cargo WASM builder"),
        ];

        let mut available = Vec::new();

        for (tool, description) in tools {
            if self.is_tool_available(tool) {
                available.push(tool.to_string());
            } else {
                println!("⚠️  {} ({}) not found - some optimizations may be skipped", tool.yellow(), description);
            }
        }

        Ok(available)
    }

    fn is_tool_available(&self, tool: &str) -> bool {
        ProcessCommand::new(tool)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn find_wasm_files(&self, directory: &str) -> Result<Vec<String>> {
        let mut wasm_files = Vec::new();
        self.find_wasm_files_recursive(directory, &mut wasm_files)?;
        Ok(wasm_files)
    }

    fn find_wasm_files_recursive(&self, dir: &str, files: &mut Vec<String>) -> Result<()> {
        let path = Path::new(dir);
        if !path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if !matches!(dir_name.as_ref(), "target" | ".git" | "node_modules") {
                    self.find_wasm_files_recursive(&path.to_string_lossy(), files)?;
                }
            } else if let Some(ext) = path.extension() {
                if ext == "wasm" {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }

        Ok(())
    }

    fn analyze_wasm_file(&self, file_path: &str) -> Result<WasmAnalysis> {
        let metadata = fs::metadata(file_path)?;
        let file_size = metadata.len();

        // Use wasm-objdump or similar to analyze the file
        let output = ProcessCommand::new("wasm-objdump")
            .args(&["-x", file_path])
            .output();

        let (function_count, export_count, import_count, has_debug_info, has_names_section) = match output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let function_count = stdout.lines().filter(|l| l.contains("func[")).count();
                let export_count = stdout.lines().filter(|l| l.contains("export")).count();
                let import_count = stdout.lines().filter(|l| l.contains("import")).count();
                let has_debug_info = stdout.contains("debug") || stdout.contains("name");
                let has_names_section = stdout.contains("name section");

                (function_count, export_count, import_count, has_debug_info, has_names_section)
            }
            _ => (0, 0, 0, false, false),
        };

        Ok(WasmAnalysis {
            file_size,
            function_count,
            export_count,
            import_count,
            has_debug_info,
            has_names_section,
            optimization_level: "unknown".to_string(),
        })
    }

    fn run_wasm_pack_build(&self, release: bool, target: &str) -> Result<String> {
        let mut args = vec!["build"];

        if release {
            args.push("--release");
        }

        args.extend(&["--target", target]);

        let output = ProcessCommand::new("wasm-pack")
            .args(&args)
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run wasm-pack: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!("wasm-pack build failed: {}", stderr)));
        }

        // Find the generated .wasm file
        let wasm_file = "pkg/package_bg.wasm";
        if Path::new(wasm_file).exists() {
            Ok(wasm_file.to_string())
        } else {
            Ok("target/wasm32-unknown-emscripten/release/*.wasm".to_string())
        }
    }

    fn optimize_with_wasm_opt(&self, input_file: &str, output_file: &str, level: &str) -> Result<()> {
        let optimization_level = match level {
            "basic" => "-O",
            "aggressive" => "-O3",
            "size" => "-Os",
            "maximum" => "-O4",
            _ => "-O2",
        };

        let output = ProcessCommand::new("wasm-opt")
            .args(&[optimization_level, input_file, "-o", output_file])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run wasm-opt: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!("wasm-opt failed: {}", stderr)));
        }

        Ok(())
    }

    fn strip_debug_info(&self, input_file: &str, output_file: &str) -> Result<()> {
        let output = ProcessCommand::new("wasm-strip")
            .args(&[input_file, "-o", output_file])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run wasm-strip: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!("wasm-strip failed: {}", stderr)));
        }

        Ok(())
    }

    fn analyze_with_twiggy(&self, file_path: &str) -> Result<String> {
        let output = ProcessCommand::new("twiggy")
            .args(&["top", "-n", "20", file_path])
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to run twiggy: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError::ExecutionFailed(format!("twiggy analysis failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn display_report(&self, report: &OptimizationReport, output_format: OutputFormat, verbose: bool) {
        match output_format {
            OutputFormat::Human => {
                println!("\n{}", "🚀 WASM Optimization Report".bold().blue());
                println!("{}", "═".repeat(50).blue());

                println!("\n📊 Size Optimization:");
                println!("  • Original: {:.2} KB", report.original_size as f64 / 1024.0);
                println!("  • Optimized: {:.2} KB", report.optimized_size as f64 / 1024.0);
                println!("  • Reduction: {:.1}%", report.reduction_percentage);

                if report.reduction_percentage > 0.0 {
                    println!("  • {}", format!("✅ Saved {:.2} KB", (report.original_size - report.optimized_size) as f64 / 1024.0).green());
                }

                println!("\n🔧 Tools Used:");
                for tool in &report.tools_used {
                    println!("  • {}", tool.green());
                }

                println!("\n⚙️  Optimization Steps:");
                for step in &report.steps_completed {
                    println!("  • {}", step.cyan());
                }

                if verbose {
                    println!("\n💡 Recommendations:");
                    for rec in &report.recommendations {
                        println!("  • {}", rec.yellow());
                    }
                }

                if report.build_time > 0.0 {
                    println!("\n⏱️  Build Time: {:.2}s", report.build_time);
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!("{:<20} {:<15} {:<15} {:<12}",
                        "Metric", "Original", "Optimized", "Reduction");
                println!("{}", "─".repeat(70));

                println!("{:<20} {:<15} {:<15} {:.1}%",
                        "File Size (KB)",
                        format!("{:.2}", report.original_size as f64 / 1024.0),
                        format!("{:.2}", report.optimized_size as f64 / 1024.0),
                        report.reduction_percentage);

                println!("{:<20} {:<15} {:<15} {:.1}%",
                        "Build Time (s)",
                        "N/A",
                        format!("{:.2}", report.build_time),
                        0.0);
            }
        }
    }
}

impl Tool for WasmOptimizeTool {
    fn name(&self) -> &'static str {
        "wasm-optimize"
    }

    fn description(&self) -> &'static str {
        "One-command WASM optimization pipeline"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Complete WebAssembly optimization pipeline with multiple tools and strategies.

EXAMPLES:
    cm tool wasm-optimize --release --aggressive
    cm tool wasm-optimize --target web --size-optimized
    cm tool wasm-optimize --analyze-only --verbose")
            .args(&[
                Arg::new("release")
                    .long("release")
                    .help("Build in release mode")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("target")
                    .long("target")
                    .short('t')
                    .help("WASM target")
                    .default_value("web")
                    .value_parser(["web", "nodejs", "bundler", "no-modules"]),
                Arg::new("optimization")
                    .long("optimization")
                    .short('O')
                    .help("Optimization level")
                    .default_value("balanced")
                    .value_parser(["none", "basic", "balanced", "aggressive", "size", "maximum"]),
                Arg::new("strip-debug")
                    .long("strip-debug")
                    .help("Strip debug information")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("analyze-only")
                    .long("analyze-only")
                    .help("Only analyze without optimization")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("input")
                    .long("input")
                    .short('i')
                    .help("Input WASM file (auto-detect if not specified)"),
                Arg::new("output")
                    .long("output")
                    .short('o')
                    .help("Output file for optimized WASM")
                    .default_value("optimized.wasm"),
                Arg::new("analyze-size")
                    .long("analyze-size")
                    .help("Analyze size with twiggy")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let release = matches.get_flag("release");
        let target = matches.get_one::<String>("target").unwrap();
        let optimization = matches.get_one::<String>("optimization").unwrap();
        let strip_debug = matches.get_flag("strip-debug");
        let analyze_only = matches.get_flag("analyze-only");
        let input_file = matches.get_one::<String>("input");
        let output_file = matches.get_one::<String>("output").unwrap();
        let analyze_size = matches.get_flag("analyze-size");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        println!("🚀 {} - Optimizing WebAssembly", "CargoMate WASM Optimize".bold().blue());

        // Check available tools
        let available_tools = self.check_wasm_tools()?;
        if available_tools.is_empty() {
            return Err(ToolError::ExecutionFailed("No WASM tools found. Install wasm-pack, wasm-opt, or similar tools".to_string()));
        }

        // Find or build WASM file
        let wasm_file = if let Some(input) = input_file {
            if !Path::new(input).exists() {
                return Err(ToolError::InvalidArguments(format!("Input file {} not found", input)));
            }
            input.clone()
        } else {
            // Try to find existing WASM files
            let wasm_files = self.find_wasm_files(".")?;
            if wasm_files.is_empty() {
                // Try to build with wasm-pack
                if available_tools.contains(&"wasm-pack".to_string()) {
                    println!("📦 No WASM file found, building with wasm-pack...");
                    self.run_wasm_pack_build(release, target)?
                } else {
                    return Err(ToolError::InvalidArguments("No WASM files found and wasm-pack not available".to_string()));
                }
            } else if wasm_files.len() == 1 {
                wasm_files[0].clone()
            } else {
                println!("📁 Multiple WASM files found:");
                for (i, file) in wasm_files.iter().enumerate() {
                    println!("  {}. {}", i + 1, file);
                }
                return Err(ToolError::InvalidArguments("Multiple WASM files found, specify --input".to_string()));
            }
        };

        // Analyze original file
        let original_analysis = self.analyze_wasm_file(&wasm_file)?;
        let original_size = original_analysis.file_size;

        if verbose {
            println!("\n📊 Original WASM Analysis:");
            println!("  • File size: {:.2} KB", original_size as f64 / 1024.0);
            println!("  • Functions: {}", original_analysis.function_count);
            println!("  • Exports: {}", original_analysis.export_count);
            println!("  • Imports: {}", original_analysis.import_count);
            println!("  • Has debug info: {}", if original_analysis.has_debug_info { "Yes" } else { "No" });
        }

        if analyze_only {
            self.display_report(&OptimizationReport {
                original_size,
                optimized_size: original_size,
                reduction_percentage: 0.0,
                steps_completed: vec!["Analysis only".to_string()],
                build_time: 0.0,
                tools_used: available_tools,
                recommendations: vec!["Use optimization flags to reduce size".to_string()],
                timestamp: chrono::Utc::now().to_rfc3339(),
            }, output_format, verbose);
            return Ok(());
        }

        // Perform optimizations
        let mut steps_completed = Vec::new();
        let mut current_file = wasm_file.clone();
        let mut optimized_size = original_size;

        // Step 1: Strip debug info if requested
        if strip_debug && available_tools.contains(&"wasm-strip".to_string()) {
            let stripped_file = format!("{}.stripped", current_file);
            match self.strip_debug_info(&current_file, &stripped_file) {
                Ok(_) => {
                    let stripped_size = fs::metadata(&stripped_file)?.len();
                    optimized_size = stripped_size;
                    current_file = stripped_file;
                    steps_completed.push(format!("Stripped debug info: {:.2} KB → {:.2} KB",
                        original_size as f64 / 1024.0, optimized_size as f64 / 1024.0));
                }
                Err(e) => {
                    println!("⚠️  Debug stripping failed: {}", e);
                }
            }
        }

        // Step 2: Optimize with wasm-opt
        if available_tools.contains(&"wasm-opt".to_string()) && optimization != "none" {
            let temp_file = format!("{}.optimized", current_file);
            match self.optimize_with_wasm_opt(&current_file, &temp_file, optimization) {
                Ok(_) => {
                    let new_size = fs::metadata(&temp_file)?.len();
                    let reduction = ((optimized_size as f64 - new_size as f64) / optimized_size as f64) * 100.0;
                    optimized_size = new_size;
                    current_file = temp_file;
                    steps_completed.push(format!("Optimized with {}: {:.1}% reduction", optimization, reduction));
                }
                Err(e) => {
                    println!("⚠️  Optimization failed: {}", e);
                }
            }
        }

        // Step 3: Analyze size with twiggy if requested
        let mut size_analysis = String::new();
        if analyze_size && available_tools.contains(&"twiggy".to_string()) {
            match self.analyze_with_twiggy(&current_file) {
                Ok(analysis) => {
                    size_analysis = analysis;
                    steps_completed.push("Size analysis completed".to_string());
                }
                Err(e) => {
                    println!("⚠️  Size analysis failed: {}", e);
                }
            }
        }

        // Move final result to output file
        if current_file != output_file.to_string() {
            fs::copy(&current_file, output_file)?;
        }

        // Generate recommendations
        let mut recommendations = Vec::new();
        let reduction_percentage = if original_size > 0 {
            ((original_size as f64 - optimized_size as f64) / original_size as f64) * 100.0
        } else {
            0.0
        };

        if reduction_percentage < 10.0 {
            recommendations.push("Consider using more aggressive optimization levels".to_string());
        }
        if original_analysis.has_debug_info {
            recommendations.push("Debug information is present - use --strip-debug for production".to_string());
        }
        if original_analysis.function_count > 1000 {
            recommendations.push("Large number of functions detected - consider code splitting".to_string());
        }

        // Create final report
        let report = OptimizationReport {
            original_size,
            optimized_size,
            reduction_percentage,
            steps_completed,
            build_time: 0.0, // Could measure this
            tools_used: available_tools,
            recommendations,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        // Display results
        self.display_report(&report, output_format, verbose);

        if verbose && !size_analysis.is_empty() {
            println!("\n📈 Size Analysis (Top Contributors):");
            for line in size_analysis.lines().take(10) {
                println!("  {}", line);
            }
        }

        if reduction_percentage > 0.0 {
            println!("\n✅ {} optimized and saved to {}", "WASM file".green(), output_file);
        }

        Ok(())
    }
}

impl Default for WasmOptimizeTool {
    fn default() -> Self {
        Self::new()
    }
}
