use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use syn::{parse_file, parse2, Item, ItemStruct, Fields, Field, Type, Attribute, Meta};
use quote::quote;
use serde_json::{Value, Map};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub struct SerdeValidatorTool;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ValidationReport {
    structs_analyzed: usize,
    fields_analyzed: usize,
    serialization_issues: Vec<SerializationIssue>,
    deserialization_issues: Vec<DeserializationIssue>,
    suggestions: Vec<String>,
    test_cases_generated: Vec<String>,
    timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializationIssue {
    struct_name: String,
    field_name: String,
    issue_type: String,
    description: String,
    severity: String,
    suggestion: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeserializationIssue {
    struct_name: String,
    field_name: String,
    issue_type: String,
    description: String,
    severity: String,
    suggestion: String,
}
#[derive(Debug, Clone)]
struct StructAnalysis {
    name: String,
    fields: Vec<FieldAnalysis>,
    has_derive_serde: bool,
    has_serialize: bool,
    has_deserialize: bool,
}
#[derive(Debug, Clone)]
struct FieldAnalysis {
    name: String,
    ty: String,
    has_serde_attrs: Vec<String>,
    is_optional: bool,
    default_value: Option<String>,
    rename: Option<String>,
    skip_serializing: bool,
    skip_deserializing: bool,
}
impl SerdeValidatorTool {
    pub fn new() -> Self {
        Self
    }
    fn analyze_rust_code(&self, file_path: &str) -> Result<Vec<StructAnalysis>> {
        let content = fs::read_to_string(file_path)?;
        let syntax = parse_file(&content)?;
        let mut structs = Vec::new();
        for item in syntax.items {
            if let Item::Struct(struct_def) = item {
                let analysis = self.analyze_struct(&struct_def)?;
                structs.push(analysis);
            }
        }
        Ok(structs)
    }
    fn analyze_struct(&self, struct_def: &syn::ItemStruct) -> Result<StructAnalysis> {
        let name = struct_def.ident.to_string();
        let mut has_derive_serde = false;
        let mut has_serialize = false;
        let mut has_deserialize = false;
        for attr in &struct_def.attrs {
            if attr.path().segments.last().unwrap().ident == "derive" {
                if let Ok(Meta::List(meta_list)) = attr.parse_args::<syn::Meta>() {
                    has_serialize = true;
                    has_deserialize = true;
                }
            }
        }
        let mut fields = Vec::new();
        if let syn::Fields::Named(named_fields) = &struct_def.fields {
            for field in &named_fields.named {
                let field_analysis = self.analyze_field(field)?;
                fields.push(field_analysis);
            }
        }
        Ok(StructAnalysis {
            name,
            fields,
            has_derive_serde,
            has_serialize,
            has_deserialize,
        })
    }
    fn analyze_field(&self, field: &Field) -> Result<FieldAnalysis> {
        let name = field.ident.as_ref().unwrap().to_string();
        let ty = quote!(# field.ty).to_string();
        let mut has_serde_attrs = Vec::new();
        let mut is_optional = false;
        let mut default_value = None;
        let mut rename = None;
        let mut skip_serializing = false;
        let mut skip_deserializing = false;
        if ty.starts_with("Option <") || ty.starts_with("std::option::Option<") {
            is_optional = true;
        }
        for attr in &field.attrs {
            if attr.path().segments.last().unwrap().ident == "serde" {
                has_serde_attrs.push(quote!(# attr).to_string());
                if let Ok(Meta::List(meta_list)) = attr.parse_args::<syn::Meta>() {
                    has_serde_attrs.push("serde_attr_detected".to_string());
                }
            }
        }
        Ok(FieldAnalysis {
            name,
            ty,
            has_serde_attrs,
            is_optional,
            default_value,
            rename,
            skip_serializing,
            skip_deserializing,
        })
    }
    fn validate_serialization(
        &self,
        structs: &[StructAnalysis],
    ) -> Vec<SerializationIssue> {
        let mut issues = Vec::new();
        for struct_analysis in structs {
            if !struct_analysis.has_serialize && !struct_analysis.has_derive_serde {
                issues
                    .push(SerializationIssue {
                        struct_name: struct_analysis.name.clone(),
                        field_name: "struct".to_string(),
                        issue_type: "missing_serialize_derive".to_string(),
                        description: "Struct does not derive Serialize".to_string(),
                        severity: "warning".to_string(),
                        suggestion: "Add #[derive(Serialize)] to the struct".to_string(),
                    });
                continue;
            }
            for field in &struct_analysis.fields {
                if field.ty.contains("std::sync::Mutex")
                    || field.ty.contains("std::rc::Rc")
                {
                    issues
                        .push(SerializationIssue {
                            struct_name: struct_analysis.name.clone(),
                            field_name: field.name.clone(),
                            issue_type: "non_serializable_type".to_string(),
                            description: format!(
                                "Field type {} is not serializable", field.ty
                            ),
                            severity: "error".to_string(),
                            suggestion: "Use a serializable type or add #[serde(skip)]"
                                .to_string(),
                        });
                }
                if field.skip_serializing && !field.is_optional
                    && field.default_value.is_none()
                {
                    issues
                        .push(SerializationIssue {
                            struct_name: struct_analysis.name.clone(),
                            field_name: field.name.clone(),
                            issue_type: "skipped_required_field".to_string(),
                            description: "Required field is skipped during serialization"
                                .to_string(),
                            severity: "warning".to_string(),
                            suggestion: "Add a default value or make the field optional"
                                .to_string(),
                        });
                }
                if field.skip_serializing != field.skip_deserializing {
                    issues
                        .push(SerializationIssue {
                            struct_name: struct_analysis.name.clone(),
                            field_name: field.name.clone(),
                            issue_type: "inconsistent_skip".to_string(),
                            description: "Field has different skip settings for serialize/deserialize"
                                .to_string(),
                            severity: "info".to_string(),
                            suggestion: "Use #[serde(skip)] for both or specify separately"
                                .to_string(),
                        });
                }
            }
        }
        issues
    }
    fn validate_deserialization(
        &self,
        structs: &[StructAnalysis],
    ) -> Vec<DeserializationIssue> {
        let mut issues = Vec::new();
        for struct_analysis in structs {
            if !struct_analysis.has_deserialize && !struct_analysis.has_derive_serde {
                issues
                    .push(DeserializationIssue {
                        struct_name: struct_analysis.name.clone(),
                        field_name: "struct".to_string(),
                        issue_type: "missing_deserialize_derive".to_string(),
                        description: "Struct does not derive Deserialize".to_string(),
                        severity: "warning".to_string(),
                        suggestion: "Add #[derive(Deserialize)] to the struct"
                            .to_string(),
                    });
                continue;
            }
            for field in &struct_analysis.fields {
                if field.ty.contains("std::sync::Mutex")
                    || field.ty.contains("std::rc::Rc")
                {
                    issues
                        .push(DeserializationIssue {
                            struct_name: struct_analysis.name.clone(),
                            field_name: field.name.clone(),
                            issue_type: "non_deserializable_type".to_string(),
                            description: format!(
                                "Field type {} is not deserializable", field.ty
                            ),
                            severity: "error".to_string(),
                            suggestion: "Use a deserializable type or add #[serde(skip)]"
                                .to_string(),
                        });
                }
                if !field.is_optional && field.default_value.is_none()
                    && field.skip_deserializing
                {
                    issues
                        .push(DeserializationIssue {
                            struct_name: struct_analysis.name.clone(),
                            field_name: field.name.clone(),
                            issue_type: "required_field_skipped".to_string(),
                            description: "Required field is skipped during deserialization"
                                .to_string(),
                            severity: "error".to_string(),
                            suggestion: "Add a default value or make the field optional"
                                .to_string(),
                        });
                }
                if let Some(rename_val) = &field.rename {
                    if rename_val.contains(" ") || rename_val.contains("-") {
                        issues
                            .push(DeserializationIssue {
                                struct_name: struct_analysis.name.clone(),
                                field_name: field.name.clone(),
                                issue_type: "complex_rename".to_string(),
                                description: format!(
                                    "Complex rename pattern: {}", rename_val
                                ),
                                severity: "info".to_string(),
                                suggestion: "Consider using simpler field names in JSON"
                                    .to_string(),
                            });
                    }
                }
            }
        }
        issues
    }
    fn generate_test_cases(&self, structs: &[StructAnalysis]) -> Vec<String> {
        let mut test_cases = Vec::new();
        for struct_analysis in structs {
            if !struct_analysis.has_serialize || !struct_analysis.has_deserialize {
                continue;
            }
            let test_name = format!(
                "test_{}_serde", struct_analysis.name.to_lowercase()
            );
            let mut test_code = format!("#[test]\nfn {}() {{\n", test_name);
            test_code
                .push_str(&format!("    let test_data = {} {{\n", struct_analysis.name));
            for field in &struct_analysis.fields {
                if field.skip_serializing || field.skip_deserializing {
                    continue;
                }
                let test_value = self.generate_test_value(field);
                test_code
                    .push_str(&format!("        {}: {},\n", field.name, test_value));
            }
            test_code.push_str("    };\n\n");
            test_code.push_str("    // Test serialization\n");
            test_code
                .push_str(
                    "    let serialized = serde_json::to_string(&test_data).unwrap();\n",
                );
            test_code.push_str("    println!(\"Serialized: {{}}\", serialized);\n\n");
            test_code.push_str("    // Test deserialization\n");
            test_code
                .push_str(
                    &format!(
                        "    let deserialized: {} = serde_json::from_str(&serialized).unwrap();\n",
                        struct_analysis.name
                    ),
                );
            test_code.push_str("    assert_eq!(test_data, deserialized);\n");
            test_code.push_str("}\n\n");
            test_cases.push(test_code);
        }
        test_cases
    }
    fn generate_test_value(&self, field: &FieldAnalysis) -> String {
        if field.is_optional {
            return "None".to_string();
        }
        if let Some(default) = &field.default_value {
            return default.clone();
        }
        if field.ty.contains("String") {
            "\"test_value\"".to_string()
        } else if field.ty.contains("i32") || field.ty.contains("i64") {
            "42".to_string()
        } else if field.ty.contains("u32") || field.ty.contains("u64") {
            "42".to_string()
        } else if field.ty.contains("bool") {
            "true".to_string()
        } else if field.ty.contains("f32") || field.ty.contains("f64") {
            "3.14".to_string()
        } else if field.ty.contains("Vec") {
            "vec![]".to_string()
        } else if field.ty.contains("HashMap") {
            "std::collections::HashMap::new()".to_string()
        } else {
            format!("{}::default()", field.ty)
        }
    }
    fn generate_suggestions(
        &self,
        structs: &[StructAnalysis],
        serialization_issues: &[SerializationIssue],
        deserialization_issues: &[DeserializationIssue],
    ) -> Vec<String> {
        let mut suggestions = Vec::new();
        if structs.iter().any(|s| !s.has_serialize && !s.has_derive_serde) {
            suggestions
                .push(
                    "Add #[derive(Serialize)] to structs that need JSON serialization"
                        .to_string(),
                );
        }
        if structs.iter().any(|s| !s.has_deserialize && !s.has_derive_serde) {
            suggestions
                .push(
                    "Add #[derive(Deserialize)] to structs that need JSON deserialization"
                        .to_string(),
                );
        }
        if serialization_issues.iter().any(|i| i.issue_type == "non_serializable_type") {
            suggestions
                .push(
                    "Replace non-serializable types (Mutex, Rc) with serializable alternatives"
                        .to_string(),
                );
        }
        if deserialization_issues
            .iter()
            .any(|i| i.issue_type == "non_deserializable_type")
        {
            suggestions
                .push(
                    "Use deserializable types or implement custom deserialization"
                        .to_string(),
                );
        }
        suggestions
            .push(
                "Use #[serde(rename_all = \"camelCase\")] for consistent field naming"
                    .to_string(),
            );
        suggestions
            .push(
                "Add #[serde(default)] to optional fields for forward compatibility"
                    .to_string(),
            );
        suggestions
            .push(
                "Use #[serde(skip)] for fields that shouldn't be serialized".to_string(),
            );
        suggestions
    }
    fn display_report(
        &self,
        report: &ValidationReport,
        output_format: OutputFormat,
        verbose: bool,
    ) {
        match output_format {
            OutputFormat::Human => {
                println!(
                    "\n🔍 {} - Serde Validation Report", "CargoMate SerdeValidator"
                    .bold().blue()
                );
                println!("{}", "═".repeat(60).blue());
                println!("\n📊 Summary:");
                println!("  • Structs Analyzed: {}", report.structs_analyzed);
                println!("  • Fields Analyzed: {}", report.fields_analyzed);
                println!(
                    "  • Serialization Issues: {}", report.serialization_issues.len()
                );
                println!(
                    "  • Deserialization Issues: {}", report.deserialization_issues
                    .len()
                );
                println!(
                    "  • Test Cases Generated: {}", report.test_cases_generated.len()
                );
                if !report.serialization_issues.is_empty() {
                    println!("\n⚠️  Serialization Issues:");
                    for issue in &report.serialization_issues {
                        let severity_icon = match issue.severity.as_str() {
                            "error" => "❌",
                            "warning" => "⚠️",
                            "info" => "ℹ️",
                            _ => "•",
                        };
                        println!(
                            "  {} {}::{} - {}", severity_icon, issue.struct_name, issue
                            .field_name, issue.description
                        );
                        if verbose {
                            println!("    💡 {}", issue.suggestion);
                        }
                    }
                }
                if !report.deserialization_issues.is_empty() {
                    println!("\n⚠️  Deserialization Issues:");
                    for issue in &report.deserialization_issues {
                        let severity_icon = match issue.severity.as_str() {
                            "error" => "❌",
                            "warning" => "⚠️",
                            "info" => "ℹ️",
                            _ => "•",
                        };
                        println!(
                            "  {} {}::{} - {}", severity_icon, issue.struct_name, issue
                            .field_name, issue.description
                        );
                        if verbose {
                            println!("    💡 {}", issue.suggestion);
                        }
                    }
                }
                if verbose && !report.test_cases_generated.is_empty() {
                    println!("\n🧪 Generated Test Cases:");
                    for test_case in &report.test_cases_generated {
                        println!("  {}", test_case.lines().next().unwrap_or(""));
                    }
                }
                if !report.suggestions.is_empty() {
                    println!("\n💡 Suggestions:");
                    for suggestion in &report.suggestions {
                        println!("  • {}", suggestion.cyan());
                    }
                }
                println!("\n✅ Validation complete!");
                if report.serialization_issues.is_empty()
                    && report.deserialization_issues.is_empty()
                {
                    println!("   All structs are properly configured for Serde!");
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!(
                    "{:<25} {:<20} {:<15} {:<15}", "Struct", "Serialization",
                    "Deserialization", "Total Issues"
                );
                println!("{}", "─".repeat(80));
                let mut struct_issues = HashMap::new();
                for issue in &report.serialization_issues {
                    let entry = struct_issues
                        .entry(&issue.struct_name)
                        .or_insert((0, 0));
                    entry.0 += 1;
                }
                for issue in &report.deserialization_issues {
                    let entry = struct_issues
                        .entry(&issue.struct_name)
                        .or_insert((0, 0));
                    entry.1 += 1;
                }
                for (struct_name, (ser_issues, deser_issues)) in struct_issues {
                    let total = ser_issues + deser_issues;
                    println!(
                        "{:<25} {:<20} {:<15} {:<15}", struct_name, ser_issues
                        .to_string(), deser_issues.to_string(), total.to_string()
                    );
                }
            }
        }
    }
}
impl Tool for SerdeValidatorTool {
    fn name(&self) -> &'static str {
        "serde-validator"
    }
    fn description(&self) -> &'static str {
        "Validate serde serialization/deserialization"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Analyze Rust structs for proper Serde serialization/deserialization setup. \
                        Detects common issues and generates test cases.

EXAMPLES:
    cm tool serde-validator --input src/models.rs
    cm tool serde-validator --workspace --generate-tests
    cm tool serde-validator --input src/api.rs --fix",
            )
            .args(
                &[
                    Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Input Rust file to analyze")
                        .required(true),
                    Arg::new("workspace")
                        .long("workspace")
                        .help("Analyze all Rust files in workspace")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("generate-tests")
                        .long("generate-tests")
                        .help("Generate test cases for validated structs")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output file for generated tests")
                        .default_value("tests/serde_tests.rs"),
                    Arg::new("fix")
                        .long("fix")
                        .help("Automatically fix simple issues")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("format")
                        .long("format")
                        .short('f')
                        .help("Serialization format to validate")
                        .default_value("json")
                        .value_parser(["json", "toml", "yaml", "bincode"]),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input");
        let workspace = matches.get_flag("workspace");
        let generate_tests = matches.get_flag("generate-tests");
        let output_file = matches.get_one::<String>("output").unwrap();
        let fix = matches.get_flag("fix");
        let format = matches.get_one::<String>("format").unwrap();
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");
        println!(
            "🔍 {} - Validating Serde Configuration", "CargoMate SerdeValidator".bold()
            .blue()
        );
        let mut all_structs = Vec::new();
        if workspace {
            let rust_files = self.find_rust_files(".")?;
            for file_path in rust_files {
                if let Ok(structs) = self.analyze_rust_code(&file_path) {
                    all_structs.extend(structs);
                }
            }
        } else if let Some(input_file) = input {
            if !Path::new(input_file).exists() {
                return Err(
                    ToolError::InvalidArguments(
                        format!("Input file {} not found", input_file),
                    ),
                );
            }
            all_structs = self.analyze_rust_code(input_file)?;
        } else {
            return Err(
                ToolError::InvalidArguments(
                    "Either specify an input file or use --workspace".to_string(),
                ),
            );
        }
        if all_structs.is_empty() {
            println!("{}", "No structs found to analyze".yellow());
            return Ok(());
        }
        if verbose {
            println!("\n📋 Found {} struct(s):", all_structs.len());
            for struct_analysis in &all_structs {
                let serde_status = if struct_analysis.has_serialize
                    && struct_analysis.has_deserialize
                {
                    "✅ Serde ready"
                } else if struct_analysis.has_serialize {
                    "📤 Serialize only"
                } else if struct_analysis.has_deserialize {
                    "📥 Deserialize only"
                } else {
                    "❌ No Serde"
                };
                println!(
                    "  • {} - {} field(s) - {}", struct_analysis.name.green(),
                    struct_analysis.fields.len(), serde_status
                );
            }
        }
        let serialization_issues = self.validate_serialization(&all_structs);
        let deserialization_issues = self.validate_deserialization(&all_structs);
        let test_cases = if generate_tests {
            self.generate_test_cases(&all_structs)
        } else {
            Vec::new()
        };
        let suggestions = self
            .generate_suggestions(
                &all_structs,
                &serialization_issues,
                &deserialization_issues,
            );
        let total_fields = all_structs.iter().map(|s| s.fields.len()).sum();
        let report = ValidationReport {
            structs_analyzed: all_structs.len(),
            fields_analyzed: total_fields,
            serialization_issues,
            deserialization_issues,
            suggestions,
            test_cases_generated: test_cases.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        if generate_tests && !test_cases.is_empty() {
            let test_file_content = self.generate_test_file(&test_cases, format);
            fs::create_dir_all(
                Path::new(output_file).parent().unwrap_or(Path::new(".")),
            )?;
            fs::write(output_file, test_file_content)?;
            println!("\n✅ Generated test file: {}", output_file);
        }
        self.display_report(&report, output_format, verbose);
        if fix
            && (!report.serialization_issues.is_empty()
                || !report.deserialization_issues.is_empty())
        {
            println!("\n🔧 Auto-fix feature not yet implemented");
            println!("   Manual fixes recommended based on the suggestions above");
        }
        Ok(())
    }
}
impl SerdeValidatorTool {
    fn find_rust_files(&self, dir: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();
        self.find_rust_files_recursive(dir, &mut files)?;
        Ok(files)
    }
    fn find_rust_files_recursive(
        &self,
        dir: &str,
        files: &mut Vec<String>,
    ) -> Result<()> {
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
                    self.find_rust_files_recursive(&path.to_string_lossy(), files)?;
                }
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }
        Ok(())
    }
    fn generate_test_file(&self, test_cases: &[String], format: &str) -> String {
        let mut content = format!(
            "//! Auto-generated Serde validation tests
//! Generated by CargoMate SerdeValidator
//! Format: {}

use serde::{{Deserialize, Serialize}};

",
            format
        );
        for test_case in test_cases {
            content.push_str(test_case);
        }
        content
    }
}
impl Default for SerdeValidatorTool {
    fn default() -> Self {
        Self::new()
    }
}