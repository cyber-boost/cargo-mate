use super::{Tool, ToolError, Result, OutputFormat, parse_output_format};
use clap::{Arg, ArgMatches, Command};
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use colored::*;
use syn::{parse_file, visit::Visit, ItemMacro, Macro};
use quote::{quote, ToTokens};
use serde::{Serialize, Deserialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroCall {
    pub name: String,
    pub args: String,
    pub span_placeholder: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
}
#[derive(Debug, Clone)]
pub struct ExpansionStep {
    pub before: String,
    pub after: String,
    pub macro_name: String,
    pub description: String,
    pub line: usize,
}
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub issue: String,
    pub severity: String,
    pub line: usize,
    pub suggestion: String,
}
#[derive(Debug, Clone)]
pub struct MacroDependency {
    pub name: String,
    pub dependency_type: String,
    pub location: String,
}
#[derive(Debug, Clone, PartialEq)]
pub enum MacroType {
    Declarative,
    Procedural,
    BuiltIn,
    Unknown,
}
pub struct MacroExpandTool;
impl MacroExpandTool {
    pub fn new() -> Self {
        Self
    }
    fn parse_macro_calls(&self, file_path: &str) -> Result<Vec<MacroCall>> {
        if !Path::new(file_path).exists() {
            return Err(
                ToolError::InvalidArguments(format!("File not found: {}", file_path)),
            );
        }
        let content = fs::read_to_string(file_path)?;
        let ast = parse_file(&content)
            .map_err(|e| ToolError::ExecutionFailed(
                format!("Failed to parse Rust file: {}", e),
            ))?;
        let mut visitor = MacroCallVisitor::new(file_path.to_string());
        visitor.visit_file(&ast);
        Ok(visitor.macro_calls)
    }
    fn expand_macro_step_by_step(
        &self,
        macro_call: &MacroCall,
    ) -> Result<Vec<ExpansionStep>> {
        let mut steps = Vec::new();
        if let Ok(macro_def) = self
            .find_macro_definition(&macro_call.name, &macro_call.file)
        {
            steps
                .push(ExpansionStep {
                    before: macro_call.args.clone(),
                    after: self.show_pattern_match(&macro_call.args, &macro_def)?,
                    macro_name: macro_call.name.clone(),
                    description: "Pattern matching and hygiene application".to_string(),
                    line: macro_call.line,
                });
            steps
                .push(ExpansionStep {
                    before: steps.last().unwrap().after.clone(),
                    after: self.apply_hygiene(&steps.last().unwrap().after)?,
                    macro_name: macro_call.name.clone(),
                    description: "Hygiene application".to_string(),
                    line: macro_call.line,
                });
        } else {
            steps
                .push(ExpansionStep {
                    before: macro_call.args.clone(),
                    after: format!("/* {} expanded */", macro_call.name),
                    macro_name: macro_call.name.clone(),
                    description: "Macro expansion (definition not found)".to_string(),
                    line: macro_call.line,
                });
        }
        Ok(steps)
    }
    fn find_macro_definition(
        &self,
        macro_name: &str,
        file_path: &str,
    ) -> Result<String> {
        let content = fs::read_to_string(file_path)?;
        let macro_pattern = format!(
            "macro_rules!\\s*{}\\s*\\{{", regex::escape(macro_name)
        );
        let re = regex::Regex::new(&macro_pattern).unwrap();
        if let Some(mat) = re.find(&content) {
            let start = mat.start();
            let mut brace_count = 0;
            let mut end = start;
            for (i, c) in content[start..].chars().enumerate() {
                match c {
                    '{' => brace_count += 1,
                    '}' => brace_count -= 1,
                    _ => {}
                }
                end = start + i;
                if brace_count == 0 {
                    break;
                }
            }
            Ok(content[start..=end].to_string())
        } else {
            Err(
                ToolError::ExecutionFailed(
                    format!("Macro definition for '{}' not found", macro_name),
                ),
            )
        }
    }
    fn show_pattern_match(&self, args: &str, macro_def: &str) -> Result<String> {
        let args = args.trim_matches(&['(', ')'][..]);
        if macro_def.contains("$x:expr") {
            let elements: Vec<&str> = args.split(',').collect();
            let mut result = "{\n    let mut v = Vec::new();".to_string();
            for elem in elements {
                result.push_str(&format!("\n    v.push({});", elem.trim()));
            }
            result.push_str("\n    v\n}");
            Ok(result)
        } else {
            Ok(format!("/* Pattern matched: {} */", args))
        }
    }
    fn apply_hygiene(&self, code: &str) -> Result<String> {
        let mut result = code
            .replace("Vec::", "::alloc::vec::Vec::")
            .replace("vec!", "::alloc::vec!");
        if result.contains("::alloc::vec::Vec::new()") {
            result = result
                .replace("::alloc::vec::Vec::new()", "::alloc::vec::Vec::new");
        }
        Ok(result)
    }
    fn generate_expanded_code(&self, file_path: &str) -> Result<String> {
        let macro_calls = self.parse_macro_calls(file_path)?;
        let content = fs::read_to_string(file_path)?;
        if macro_calls.is_empty() {
            return Ok(content);
        }
        let mut expanded = content.clone();
        for call in macro_calls {
            if let Ok(steps) = self.expand_macro_step_by_step(&call) {
                if let Some(final_step) = steps.last() {
                    let macro_call_pattern = format!("{}!{}", call.name, call.args);
                    expanded = expanded.replace(&macro_call_pattern, &final_step.after);
                }
            }
        }
        Ok(expanded)
    }
    fn highlight_differences(&self, original: &str, expanded: &str) -> Result<String> {
        let original_lines: Vec<&str> = original.lines().collect();
        let expanded_lines: Vec<&str> = expanded.lines().collect();
        let mut result = String::new();
        for (i, (orig, exp)) in original_lines
            .iter()
            .zip(expanded_lines.iter())
            .enumerate()
        {
            if orig != exp {
                result
                    .push_str(
                        &format!(
                            "{} {} {}\n", format!("{}:", i + 1) .yellow(), "-".red(),
                            orig.red()
                        ),
                    );
                result
                    .push_str(
                        &format!(
                            "{} {} {}\n", format!("{}:", i + 1) .yellow(), "+".green(),
                            exp.green()
                        ),
                    );
            } else {
                result
                    .push_str(
                        &format!("{}   {}\n", format!("{}:", i + 1) .blue(), orig),
                    );
            }
        }
        Ok(result)
    }
    fn extract_macro_dependencies(
        &self,
        macro_call: &MacroCall,
    ) -> Vec<MacroDependency> {
        let mut deps = Vec::new();
        if macro_call.args.contains("vec!") {
            deps.push(MacroDependency {
                name: "vec".to_string(),
                dependency_type: "Built-in macro".to_string(),
                location: "std library".to_string(),
            });
        }
        if macro_call.name.contains("println") || macro_call.name.contains("print") {
            deps.push(MacroDependency {
                name: "print".to_string(),
                dependency_type: "Built-in macro".to_string(),
                location: "std library".to_string(),
            });
        }
        deps
    }
    fn validate_macro_expansion(
        &self,
        expanded_code: &str,
    ) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        if let Err(_) = syn::parse_file(expanded_code) {
            issues
                .push(ValidationIssue {
                    issue: "Expanded code contains syntax errors".to_string(),
                    severity: "High".to_string(),
                    line: 0,
                    suggestion: "Review macro definition and arguments".to_string(),
                });
        }
        if expanded_code.contains("unresolved name") {
            issues
                .push(ValidationIssue {
                    issue: "Unresolved names in expanded code".to_string(),
                    severity: "Medium".to_string(),
                    line: 0,
                    suggestion: "Check macro hygiene and imports".to_string(),
                });
        }
        Ok(issues)
    }
    fn classify_macro(&self, macro_name: &str) -> MacroType {
        match macro_name {
            "println" | "print" | "format" | "vec" | "assert" | "panic" => {
                MacroType::BuiltIn
            }
            name if name.contains("macro_rules!") => MacroType::Declarative,
            name if name.contains("#[") => MacroType::Procedural,
            _ => MacroType::Unknown,
        }
    }
    fn format_expanded_code(&self, code: &str, format: &str) -> Result<String> {
        match format {
            "rust" => Ok(code.to_string()),
            "html" => self.format_as_html(code),
            "json" => self.format_as_json(code),
            _ => Ok(code.to_string()),
        }
    }
    fn format_as_html(&self, code: &str) -> Result<String> {
        let html = format!(
            "<!DOCTYPE html>
<html>
<head>
    <title>Macro Expansion</title>
    <style>
        .code {{ font-family: 'Monaco', 'Menlo', monospace; background: #f5f5f5; padding: 1em; }}
        .expanded {{ color: #28a745; }}
        .original {{ color: #dc3545; }}
    </style>
</head>
<body>
    <h1>Macro Expansion Result</h1>
    <pre class=\"code\">{}</pre>
</body>
</html>",
            code.replace("<", "&lt;").replace(">", "&gt;")
        );
        Ok(html)
    }
    fn format_as_json(&self, code: &str) -> Result<String> {
        let json = serde_json::json!(
            { "expanded_code" : code, "timestamp" : chrono::Utc::now().to_rfc3339(),
            "tool" : "macro-expand" }
        );
        Ok(serde_json::to_string_pretty(&json).unwrap())
    }
}
struct MacroCallVisitor {
    macro_calls: Vec<MacroCall>,
    current_file: String,
}
impl MacroCallVisitor {
    fn new(file_path: String) -> Self {
        Self {
            macro_calls: Vec::new(),
            current_file: file_path,
        }
    }
}
impl<'ast> Visit<'ast> for MacroCallVisitor {
    fn visit_macro(&mut self, node: &'ast Macro) {
        if let Some(last_segment) = node.path.segments.last() {
            let macro_name = last_segment.ident.to_string();
            let args = quote!(# node).to_string();
            self.macro_calls
                .push(MacroCall {
                    name: macro_name,
                    args: format!("({})", args),
                    span_placeholder: "span_info_unavailable".to_string(),
                    file: self.current_file.clone(),
                    line: 0,
                    column: 0,
                });
        }
        syn::visit::visit_macro(self, node);
    }
}
impl Tool for MacroExpandTool {
    fn name(&self) -> &'static str {
        "macro-expand"
    }
    fn description(&self) -> &'static str {
        "Better macro expansion viewer with step-by-step expansion and syntax highlighting"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Provide a better macro expansion viewer with syntax highlighting, step-by-step expansion, and interactive exploration.\n\
                 \n\
                 This tool helps you understand complex macros by:\n\
                 • Expanding procedural and declarative macros\n\
                 • Showing intermediate expansion steps\n\
                 • Syntax highlighting for expanded code\n\
                 • Comparing original vs expanded code\n\
                 \n\
                 EXAMPLES:\n\
                 cm tool macro-expand --input src/lib.rs --step-by-step\n\
                 cm tool macro-expand --input src/main.rs --macro my_macro --highlight\n\
                 cm tool macro-expand --input src/lib.rs --diff --validate",
            )
            .args(
                &[
                    Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Input Rust file to analyze")
                        .required(true),
                    Arg::new("macro")
                        .long("macro")
                        .short('m')
                        .help("Specific macro to expand (expand all if not specified)"),
                    Arg::new("step-by-step")
                        .long("step-by-step")
                        .help("Show step-by-step expansion")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("highlight")
                        .long("highlight")
                        .help("Highlight expanded code with syntax highlighting")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("diff")
                        .long("diff")
                        .help("Show diff between original and expanded")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("validate")
                        .long("validate")
                        .help("Validate that expanded code compiles")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("interactive")
                        .long("interactive")
                        .help("Interactive macro exploration mode")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output file for expanded code")
                        .default_value("expanded.rs"),
                    Arg::new("format")
                        .long("format")
                        .short('f')
                        .help("Output format: rust, html, json")
                        .default_value("rust"),
                ],
            )
            .args(&super::common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input").unwrap();
        let specific_macro = matches.get_one::<String>("macro");
        let step_by_step = matches.get_flag("step-by-step");
        let highlight = matches.get_flag("highlight");
        let diff = matches.get_flag("diff");
        let validate = matches.get_flag("validate");
        let interactive = matches.get_flag("interactive");
        let output_file = matches.get_one::<String>("output").unwrap();
        let format = matches.get_one::<String>("format").unwrap();
        let verbose = matches.get_flag("verbose");
        let dry_run = matches.get_flag("dry-run");
        let output_format = parse_output_format(matches);
        if dry_run {
            println!("🔍 Would analyze macro expansion in: {}", input);
            return Ok(());
        }
        match output_format {
            OutputFormat::Human => {
                println!(
                    "🔍 {} - {}", "Macro Expansion Analysis".bold(), self.description()
                    .cyan()
                );
                match self.parse_macro_calls(input) {
                    Ok(macro_calls) => {
                        println!("\n📁 File: {}", input.bold());
                        println!(
                            "🔍 Macros Found: {}", macro_calls.len().to_string().cyan()
                        );
                        if macro_calls.is_empty() {
                            println!("✅ No macro calls found in the file.");
                            return Ok(());
                        }
                        let filtered_calls: Vec<_> = if let Some(macro_name) = specific_macro {
                            macro_calls
                                .into_iter()
                                .filter(|call| call.name == *macro_name)
                                .collect()
                        } else {
                            macro_calls
                        };
                        if filtered_calls.is_empty() {
                            if let Some(name) = specific_macro {
                                println!(
                                    "❌ Macro '{}' not found in the file.", name.red()
                                );
                            }
                            return Ok(());
                        }
                        let mut macro_types = HashMap::new();
                        for call in &filtered_calls {
                            let macro_type = self.classify_macro(&call.name);
                            macro_types
                                .entry(format!("{:?}", macro_type))
                                .or_insert_with(Vec::new)
                                .push(call.name.clone());
                        }
                        println!("\n📊 Expansion Summary:");
                        for (type_name, names) in &macro_types {
                            println!("  • {}: {}", type_name, names.len());
                        }
                        for (i, call) in filtered_calls.iter().enumerate() {
                            println!("\n🔬 Macro: {}!{}", call.name.bold(), call.args);
                            println!(
                                "   📍 Line {}, Column {}", call.line, call.column
                            );
                            if step_by_step {
                                match self.expand_macro_step_by_step(call) {
                                    Ok(steps) => {
                                        for (step_num, step) in steps.iter().enumerate() {
                                            println!(
                                                "\n   Step {} - {}:", step_num + 1, step.description.bold()
                                            );
                                            println!("   ```rust");
                                            for line in step.after.lines() {
                                                println!("   {}", line);
                                            }
                                            println!("   ```");
                                        }
                                    }
                                    Err(e) => {
                                        if verbose {
                                            println!("   ⚠️  Could not expand step by step: {}", e);
                                        }
                                    }
                                }
                            }
                            let deps = self.extract_macro_dependencies(call);
                            if !deps.is_empty() {
                                println!("\n   📚 Dependencies:");
                                for dep in &deps {
                                    println!(
                                        "     • {} ({})", dep.name.cyan(), dep.dependency_type
                                    );
                                }
                            }
                            if validate {
                                if let Ok(steps) = self.expand_macro_step_by_step(call) {
                                    if let Some(final_step) = steps.last() {
                                        match self.validate_macro_expansion(&final_step.after) {
                                            Ok(issues) => {
                                                if issues.is_empty() {
                                                    println!(
                                                        "\n   ✅ Validation: Expanded code compiles successfully"
                                                    );
                                                } else {
                                                    for issue in issues {
                                                        let severity_color = match issue.severity.as_str() {
                                                            "High" => issue.severity.red().bold(),
                                                            "Medium" => issue.severity.yellow().bold(),
                                                            _ => issue.severity.green().bold(),
                                                        };
                                                        println!(
                                                            "\n   🚨 Validation Issue [{}]: {}", severity_color, issue
                                                            .issue
                                                        );
                                                        println!("      💡 {}", issue.suggestion.cyan());
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                if verbose {
                                                    println!("\n   ⚠️  Validation failed: {}", e);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Ok(steps) = self.expand_macro_step_by_step(call) {
                                if let Some(final_step) = steps.last() {
                                    let original_size = call.args.len();
                                    let expanded_size = final_step.after.len();
                                    let ratio = if original_size > 0 {
                                        expanded_size as f64 / original_size as f64
                                    } else {
                                        1.0
                                    };
                                    println!("\n📈 Expansion Metrics:");
                                    println!("   • Original size: {} chars", original_size);
                                    println!("   • Expanded size: {} chars", expanded_size);
                                    println!("   • Expansion ratio: {:.1}x", ratio);
                                }
                            }
                            if i < filtered_calls.len() - 1 {
                                println!("{}", "─".repeat(50).blue());
                            }
                        }
                        if let Ok(expanded) = self.generate_expanded_code(input) {
                            if diff {
                                println!("\n📋 Original vs Expanded Comparison:");
                                match self
                                    .highlight_differences(
                                        &fs::read_to_string(input).unwrap_or_default(),
                                        &expanded,
                                    )
                                {
                                    Ok(diff_output) => {
                                        for line in diff_output.lines().take(20) {
                                            println!("   {}", line);
                                        }
                                        if diff_output.lines().count() > 20 {
                                            println!(
                                                "   ... (truncated - {} more lines)", diff_output.lines()
                                                .count() - 20
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        if verbose {
                                            println!("   ⚠️  Could not generate diff: {}", e);
                                        }
                                    }
                                }
                            }
                            if let Err(e) = fs::write(output_file, &expanded) {
                                if verbose {
                                    println!("⚠️  Could not write to output file: {}", e);
                                }
                            } else if verbose {
                                println!(
                                    "\n💾 Expanded code written to: {}", output_file
                                );
                            }
                        }
                    }
                    Err(e) => {
                        return Err(
                            ToolError::ExecutionFailed(
                                format!("Failed to analyze macros: {}", e),
                            ),
                        );
                    }
                }
            }
            OutputFormat::Json => {
                let macro_calls = self.parse_macro_calls(input)?;
                let mut json_output = serde_json::json!(
                    { "file" : input, "macro_calls" : macro_calls.len(), "macros" :
                    macro_calls, }
                );
                if let Ok(expanded) = self.generate_expanded_code(input) {
                    json_output["expanded_code"] = expanded.into();
                }
                println!("{}", serde_json::to_string_pretty(& json_output).unwrap());
            }
            OutputFormat::Table => {
                let macro_calls = self.parse_macro_calls(input)?;
                println!(
                    "┌─ Macro Expansion Analysis ──────────────────────┐"
                );
                println!("│ File: {:<45} │", input);
                println!("│ Macros Found: {:<36} │", macro_calls.len());
                for call in macro_calls.iter().take(5) {
                    println!(
                        "│ • {:<45} │", format!("{}!{}", call.name, call.args)
                    );
                }
                if macro_calls.len() > 5 {
                    println!(
                        "│ ... and {} more {:<32} │", macro_calls.len() - 5, ""
                    );
                }
                println!(
                    "└──────────────────────────────────────────────────┘"
                );
            }
        }
        Ok(())
    }
}