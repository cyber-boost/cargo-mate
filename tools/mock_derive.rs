use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::collections::HashSet;
use std::path::Path;
use std::fs;
use syn::{parse_file, ItemTrait, TraitItem, FnArg, Pat, Type};
use quote::quote;
use proc_macro2::TokenStream;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct MockDeriveTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraitInfo {
    name: String,
    methods: Vec<MethodInfo>,
    is_unsafe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MethodInfo {
    name: String,
    return_type: Option<String>,
    params: Vec<ParamInfo>,
    is_async: bool,
    is_unsafe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParamInfo {
    name: String,
    ty: String,
    is_self: bool,
}

impl MockDeriveTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_trait_from_file(&self, file_path: &str) -> Result<Vec<TraitInfo>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::IoError(e))?;

        let syntax = parse_file(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse Rust file: {}", e)))?;

        let mut traits = Vec::new();

        for item in syntax.items {
            if let syn::Item::Trait(trait_def) = item {
                let trait_info = self.parse_trait(&trait_def)?;
                traits.push(trait_info);
            }
        }

        Ok(traits)
    }

    fn parse_trait(&self, trait_def: &syn::ItemTrait) -> Result<TraitInfo> {
        let name = trait_def.ident.to_string();
        let is_unsafe = trait_def.unsafety.is_some();

        let mut methods = Vec::new();

        for item in &trait_def.items {
            if let syn::TraitItem::Fn(method) = item {
                let method_info = self.parse_trait_method(method)?;
                methods.push(method_info);
            }
        }

        Ok(TraitInfo {
            name,
            methods,
            is_unsafe,
        })
    }

    fn parse_trait_method(&self, method: &syn::TraitItemFn) -> Result<MethodInfo> {
        let name = method.sig.ident.to_string();
        let is_async = method.sig.asyncness.is_some();
        let is_unsafe = method.sig.unsafety.is_some();

        let return_type = if let syn::ReturnType::Type(_, ty) = &method.sig.output {
            Some(quote!(#ty).to_string())
        } else {
            None
        };

        let mut params = Vec::new();

        for arg in &method.sig.inputs {
            match arg {
                syn::FnArg::Receiver(receiver) => {
                    let self_type = if receiver.reference.is_some() {
                        if receiver.mutability.is_some() {
                            "&mut self"
                        } else {
                            "&self"
                        }
                    } else {
                        "self"
                    };
                    params.push(ParamInfo {
                        name: "self".to_string(),
                        ty: self_type.to_string(),
                        is_self: true,
                    });
                }
                syn::FnArg::Typed(pat_type) => {
                    if let syn::Pat::Ident(pat_ident) = &*pat_type.pat {
                        let param_name = pat_ident.ident.to_string();
                        let param_type = quote!(#pat_type.ty).to_string();
                        params.push(ParamInfo {
                            name: param_name,
                            ty: param_type,
                            is_self: false,
                        });
                    }
                }
            }
        }

        Ok(MethodInfo {
            name,
            return_type,
            params,
            is_async,
            is_unsafe,
        })
    }

    fn generate_mock_struct(&self, trait_info: &TraitInfo) -> String {
        let struct_name = format!("Mock{}", trait_info.name);
        let mut mock_code = format!("/// Auto-generated mock for trait `{}`\n", trait_info.name);
        mock_code.push_str("#[derive(Debug, Clone, Default)]\n");
        mock_code.push_str(&format!("pub struct {} {{\n", struct_name));

        // Generate call count fields
        for method in &trait_info.methods {
            mock_code.push_str(&format!("    pub {}_call_count: std::sync::atomic::AtomicUsize,\n", method.name));
        }

        // Generate return value storage
        for method in &trait_info.methods {
            if let Some(return_type) = &method.return_type {
                mock_code.push_str(&format!("    pub {}_return_value: Option<{}>,\n", method.name, return_type));
            }
        }

        mock_code.push_str("}\n\n");

        // Generate mock implementation
        mock_code.push_str(&format!("impl {} for {} {{\n", trait_info.name, struct_name));

        for method in &trait_info.methods {
            mock_code.push_str(&self.generate_mock_method(method));
        }

        mock_code.push_str("}\n\n");

        // Generate helper methods
        mock_code.push_str(&self.generate_helper_methods(trait_info));

        mock_code
    }

    fn generate_mock_method(&self, method: &MethodInfo) -> String {
        let mut method_code = String::new();

        // Method signature
        if method.is_async {
            method_code.push_str("    async ");
        }

        if method.is_unsafe {
            method_code.push_str("unsafe ");
        }

        method_code.push_str(&format!("fn {}(", method.name));

        // Parameters
        let param_strs: Vec<String> = method.params.iter()
            .map(|param| {
                if param.is_self {
                    param.ty.clone()
                } else {
                    format!("{}: {}", param.name, param.ty)
                }
            })
            .collect();
        method_code.push_str(&param_strs.join(", "));
        method_code.push_str(")");

        // Return type
        if let Some(return_type) = &method.return_type {
            method_code.push_str(&format!(" -> {}", return_type));
        }

        method_code.push_str(" {\n");

        // Method body - increment call count
        method_code.push_str(&format!("        self.{}_call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);\n", method.name));

        // Return value
        if let Some(return_type) = &method.return_type {
            method_code.push_str(&format!("        self.{}_return_value.unwrap_or_default()\n", method.name));
        } else {
            method_code.push_str("        ()\n");
        }

        method_code.push_str("    }\n\n");

        method_code
    }

    fn generate_helper_methods(&self, trait_info: &TraitInfo) -> String {
        let struct_name = format!("Mock{}", trait_info.name);
        let mut helper_code = String::new();

        helper_code.push_str(&format!("impl {} {{\n", struct_name));

        // Call count getters
        for method in &trait_info.methods {
            helper_code.push_str(&format!("    pub fn {}_called(&self) -> bool {{\n", method.name));
            helper_code.push_str(&format!("        self.{}_call_count.load(std::sync::atomic::Ordering::Relaxed) > 0\n", method.name));
            helper_code.push_str("    }\n\n");

            helper_code.push_str(&format!("    pub fn {}_call_count(&self) -> usize {{\n", method.name));
            helper_code.push_str(&format!("        self.{}_call_count.load(std::sync::atomic::Ordering::Relaxed)\n", method.name));
            helper_code.push_str("    }\n\n");
        }

        // Return value setters
        for method in &trait_info.methods {
            if let Some(return_type) = &method.return_type {
                helper_code.push_str(&format!("    pub fn set_{}_return(&mut self, value: {}) {{\n", method.name, return_type));
                helper_code.push_str(&format!("        self.{}_return_value = Some(value);\n", method.name));
                helper_code.push_str("    }\n\n");
            }
        }

        // Reset method
        helper_code.push_str("    pub fn reset(&mut self) {\n");
        for method in &trait_info.methods {
            helper_code.push_str(&format!("        self.{}_call_count.store(0, std::sync::atomic::Ordering::Relaxed);\n", method.name));
            if method.return_type.is_some() {
                helper_code.push_str(&format!("        self.{}_return_value = None;\n", method.name));
            }
        }
        helper_code.push_str("    }\n");

        helper_code.push_str("}\n\n");

        helper_code
    }

    fn display_analysis(&self, traits: &[TraitInfo], output_format: OutputFormat, verbose: bool) {
        match output_format {
            OutputFormat::Human => {
                println!("\n{}", "🎭 Mock Generation Analysis Report".bold().blue());
                println!("{}", "═".repeat(50).blue());

                println!("\n📋 Found {} trait(s):", traits.len());
                for trait_info in traits {
                    println!("  • {} - {} method(s)", trait_info.name.green(), trait_info.methods.len());

                    if verbose {
                        for method in &trait_info.methods {
                            let async_str = if method.is_async { "async " } else { "" };
                            let unsafe_str = if method.is_unsafe { "unsafe " } else { "" };
                            println!("    - {}{}{}({} params)",
                                async_str.bright_blue(),
                                unsafe_str.red(),
                                method.name,
                                method.params.len());
                        }
                    }
                }

                if !traits.is_empty() {
                    println!("\n💡 Generated mock structs:");
                    for trait_info in traits {
                        println!("  • Mock{} - Ready for testing", trait_info.name.green());
                    }
                }
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(traits)
                    .unwrap_or_else(|_| "[]".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!("{:<25} {:<10} {:<15} {:<10}",
                        "Trait", "Methods", "Unsafe", "Async");
                println!("{}", "─".repeat(70));

                for trait_info in traits {
                    let async_count = trait_info.methods.iter().filter(|m| m.is_async).count();
                    let unsafe_count = trait_info.methods.iter().filter(|m| m.is_unsafe).count();
                    println!("{:<25} {:<10} {:<15} {:<10}",
                            trait_info.name,
                            trait_info.methods.len().to_string(),
                            unsafe_count.to_string(),
                            async_count.to_string());
                }
            }
        }
    }
}

impl Tool for MockDeriveTool {
    fn name(&self) -> &'static str {
        "mock-derive"
    }

    fn description(&self) -> &'static str {
        "Auto-generate mock implementations for traits"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Generate mock implementations for Rust traits automatically. \
                        This tool analyzes trait definitions and creates mock structs \
                        with call counting, return value injection, and testing helpers.

EXAMPLES:
    cm tool mock-derive src/lib.rs --output mocks/
    cm tool mock-derive src/traits.rs --traits UserService,Database
    cm tool mock-derive --workspace --verbose")
            .args(&[
                Arg::new("input")
                    .help("Input Rust file(s) containing traits")
                    .value_name("FILE")
                    .index(1),
                Arg::new("output")
                    .long("output")
                    .short('o')
                    .help("Output directory for generated mocks")
                    .default_value("mocks/"),
                Arg::new("traits")
                    .long("traits")
                    .short('t')
                    .help("Specific traits to mock (comma-separated)"),
                Arg::new("workspace")
                    .long("workspace")
                    .help("Generate mocks for all traits in workspace")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("force")
                    .long("force")
                    .short('f')
                    .help("Overwrite existing mock files")
                    .action(clap::ArgAction::SetTrue),
                Arg::new("include-async")
                    .long("include-async")
                    .help("Include async trait support")
                    .action(clap::ArgAction::SetTrue),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input = matches.get_one::<String>("input");
        let output_dir = matches.get_one::<String>("output").unwrap();
        let traits_filter = matches.get_one::<String>("traits")
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect::<HashSet<_>>())
            .unwrap_or_default();
        let workspace = matches.get_flag("workspace");
        let force = matches.get_flag("force");
        let include_async = matches.get_flag("include-async");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        // Create output directory
        std::fs::create_dir_all(output_dir)
            .map_err(|e| ToolError::IoError(e))?;

        let mut all_traits = Vec::new();

        if workspace {
            // Find all .rs files in workspace
            let workspace_files = self.find_workspace_files()?;
            for file_path in workspace_files {
                if let Ok(traits) = self.parse_trait_from_file(&file_path) {
                    for trait_info in traits {
                        if traits_filter.is_empty() || traits_filter.contains(&trait_info.name) {
                            all_traits.push((file_path.clone(), trait_info));
                        }
                    }
                }
            }
        } else if let Some(input_file) = input {
            let traits = self.parse_trait_from_file(input_file)?;
            for trait_info in traits {
                if traits_filter.is_empty() || traits_filter.contains(&trait_info.name) {
                    all_traits.push((input_file.to_string(), trait_info));
                }
            }
        } else {
            return Err(ToolError::InvalidArguments("Either specify an input file or use --workspace".to_string()));
        }

        if all_traits.is_empty() {
            println!("{}", "No traits found matching criteria".yellow());
            return Ok(());
        }

        // Generate mocks
        let mut generated_files = Vec::new();

        for (file_path, trait_info) in &all_traits {
            let mock_code = self.generate_mock_struct(trait_info);

            // Generate output filename
            let file_name = format!("mock_{}.rs", trait_info.name.to_lowercase());
            let output_path = Path::new(output_dir).join(file_name);

            // Check if file exists
            if !force && output_path.exists() {
                println!("⚠️  Skipping {} (use --force to overwrite)", output_path.display());
                continue;
            }

            // Write mock file
            fs::write(&output_path, mock_code)
                .map_err(|e| ToolError::IoError(e))?;

            generated_files.push((trait_info.name.clone(), output_path.display().to_string()));

            if verbose {
                println!("✅ Generated mock for trait {} -> {}", trait_info.name, output_path.display());
            }
        }

        // Extract just the trait info for display
        let trait_infos: Vec<TraitInfo> = all_traits.into_iter().map(|(_, info)| info).collect();

        // Display results
        self.display_analysis(&trait_infos, output_format, verbose);

        if !generated_files.is_empty() {
            println!("\n🎭 Generated {} mock(s):", generated_files.len());
            for (trait_name, file_path) in &generated_files {
                println!("  • Mock{} -> {}", trait_name.green(), file_path);
            }

            println!("\n💡 Add to your tests:");
            println!("  ```rust");
            println!("  use mocks::MockMyTrait;");
            println!("  let mut mock = MockMyTrait::default();");
            println!("  mock.set_my_method_return(42);");
            println!("  ```");
        }

        Ok(())
    }
}

impl MockDeriveTool {
    fn find_workspace_files(&self) -> Result<Vec<String>> {
        let mut files = Vec::new();

        // Find Cargo.toml to determine workspace structure
        let cargo_toml_path = "Cargo.toml";
        if !Path::new(cargo_toml_path).exists() {
            return Err(ToolError::InvalidArguments("Not in a Rust project (Cargo.toml not found)".to_string()));
        }

        // Simple approach: find all .rs files in src/ directory
        self.find_rs_files("src", &mut files)?;

        Ok(files)
    }

    fn find_rs_files(&self, dir: &str, files: &mut Vec<String>) -> Result<()> {
        let path = Path::new(dir);
        if !path.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Skip common directories we don't want to analyze
                let dir_name = path.file_name().unwrap_or_default().to_string_lossy();
                if !matches!(dir_name.as_ref(), "target" | ".git" | "mocks" | "tests") {
                    self.find_rs_files(&path.to_string_lossy(), files)?;
                }
            } else if let Some(ext) = path.extension() {
                if ext == "rs" {
                    files.push(path.to_string_lossy().to_string());
                }
            }
        }

        Ok(())
    }
}

impl Default for MockDeriveTool {
    fn default() -> Self {
        Self::new()
    }
}
