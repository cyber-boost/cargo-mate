use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use syn::{parse_file, FnArg, Pat, ReturnType, visit::Visit};
use quote::ToTokens;

#[derive(Debug, Clone)]
pub struct TestGenTool;

#[derive(Debug)]
struct FunctionInfo {
    name: String,
    params: Vec<ParamInfo>,
    return_type: Option<String>,
    is_async: bool,
    visibility: String,
}

#[derive(Debug)]
struct ParamInfo {
    name: String,
    ty: String,
    is_reference: bool,
}

#[derive(Debug, serde::Serialize)]
struct GeneratedTest {
    function_name: String,
    test_name: String,
    test_code: String,
}

impl TestGenTool {
    pub fn new() -> Self {
        Self
    }

    fn parse_rust_file(&self, file_path: &Path) -> Result<Vec<FunctionInfo>> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to read file: {}", e)))?;

        let syntax = parse_file(&content)
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse Rust code: {}", e)))?;

        let mut visitor = FunctionVisitor::new();
        visitor.visit_file(&syntax);

        Ok(visitor.functions)
    }

    fn generate_test_for_function(&self, func: &FunctionInfo, test_type: &str) -> GeneratedTest {
        let test_name = format!("test_{}_{}", func.name, test_type);

        let test_code = match test_type {
            "unit" => self.generate_unit_test(func),
            "integration" => self.generate_integration_test(func),
            "property" => self.generate_property_test(func),
            _ => self.generate_unit_test(func),
        };

        GeneratedTest {
            function_name: func.name.clone(),
            test_name,
            test_code,
        }
    }

    fn generate_unit_test(&self, func: &FunctionInfo) -> String {
        let mut code = format!("/// Unit test for `{}`\n", func.name);
        code.push_str("#[test]\n");

        if func.is_async {
            code.push_str("#[tokio::test]\n");
        }

        code.push_str(&format!("fn {}() {{\n", self.snake_to_pascal(&format!("test_{}", func.name))));

        // Generate mock parameters
        for param in &func.params {
            let mock_value = self.generate_mock_value(&param.ty);
            code.push_str(&format!("    let {} = {};\n", param.name, mock_value));
        }

        // Generate function call
        let params_str = func.params.iter()
            .map(|p| if p.is_reference { format!("&{}", p.name) } else { p.name.clone() })
            .collect::<Vec<_>>()
            .join(", ");

        if func.is_async {
            code.push_str(&format!("    let result = {}({}).await;\n", func.name, params_str));
        } else {
            code.push_str(&format!("    let result = {}({});\n", func.name, params_str));
        }

        // Generate assertions
        if let Some(return_type) = &func.return_type {
            if return_type.contains("Result") {
                code.push_str("    assert!(result.is_ok());\n");
            } else if return_type.contains("Option") {
                code.push_str("    assert!(result.is_some());\n");
            } else if return_type == "bool" {
                code.push_str("    assert!(result);\n");
            } else if return_type.contains("Vec") || return_type.contains("HashMap") {
                code.push_str("    assert!(!result.is_empty());\n");
            } else {
                code.push_str("    // Add your assertions here\n");
                code.push_str("    assert!(true); // Placeholder assertion\n");
            }
        } else {
            code.push_str("    // Function returns nothing - add your test logic here\n");
            code.push_str("    assert!(true); // Placeholder assertion\n");
        }

        code.push_str("}\n");

        code
    }

    fn generate_integration_test(&self, func: &FunctionInfo) -> String {
        let mut code = format!("/// Integration test for `{}`\n", func.name);
        code.push_str("#[test]\n");

        if func.is_async {
            code.push_str("#[tokio::test]\n");
        }

        code.push_str(&format!("fn {}() {{\n", self.snake_to_pascal(&format!("integration_test_{}", func.name))));

        // Setup code for integration test
        code.push_str("    // Setup test environment\n");

        // Generate parameters with more realistic values
        for param in &func.params {
            let mock_value = self.generate_integration_mock_value(&param.ty);
            code.push_str(&format!("    let {} = {};\n", param.name, mock_value));
        }

        // Generate function call
        let params_str = func.params.iter()
            .map(|p| if p.is_reference { format!("&{}", p.name) } else { p.name.clone() })
            .collect::<Vec<_>>()
            .join(", ");

        if func.is_async {
            code.push_str(&format!("    let result = {}({}).await;\n", func.name, params_str));
        } else {
            code.push_str(&format!("    let result = {}({});\n", func.name, params_str));
        }

        // More comprehensive assertions for integration tests
        code.push_str("    // Verify the result\n");
        if let Some(return_type) = &func.return_type {
            if return_type.contains("Result") {
                code.push_str("    match result {\n");
                code.push_str("        Ok(value) => {\n");
                code.push_str("            // Add your success case assertions here\n");
                code.push_str("            assert!(true); // Placeholder\n");
                code.push_str("        }\n");
                code.push_str("        Err(e) => {\n");
                code.push_str("            panic!(\"Integration test failed: {}\", e);\n");
                code.push_str("        }\n");
                code.push_str("    }\n");
            } else {
                code.push_str("    // Add your integration test assertions here\n");
                code.push_str("    assert!(true); // Placeholder assertion\n");
            }
        }

        code.push_str("}\n");

        code
    }

    fn generate_property_test(&self, func: &FunctionInfo) -> String {
        let mut code = format!("/// Property-based test for `{}`\n", func.name);
        code.push_str("#[cfg(test)]\n");
        code.push_str("mod property_tests {\n");
        code.push_str("    use super::*;\n");
        code.push_str("    use proptest::prelude::*;\n\n");

        // Check if function has numeric parameters for property testing
        let has_numeric_params = func.params.iter().any(|p| {
            let ty = p.ty.to_lowercase();
            ty.contains("i32") || ty.contains("i64") || ty.contains("u32") || ty.contains("u64") ||
            ty.contains("f32") || ty.contains("f64") || ty.contains("usize") || ty.contains("isize")
        });

        if has_numeric_params {
            code.push_str(&format!("    proptest! {{\n"));
            code.push_str(&format!("        #[test]\n"));
            code.push_str(&format!("        fn {}({}) {{\n", func.name, self.generate_proptest_params(&func.params)));

            // Generate property test body
            let params_str = func.params.iter()
                .map(|p| if p.is_reference { format!("&{}", p.name) } else { p.name.clone() })
                .collect::<Vec<_>>()
                .join(", ");

            if func.is_async {
                code.push_str(&format!("            let result = {}({}).await;\n", func.name, params_str));
            } else {
                code.push_str(&format!("            let result = {}({});\n", func.name, params_str));
            }

            // Add property-based assertions
            if let Some(return_type) = &func.return_type {
                if return_type.contains("Result") {
                    code.push_str("            prop_assert!(result.is_ok());\n");
                } else if return_type.contains("bool") {
                    code.push_str("            // Add property-based assertions for boolean results\n");
                    code.push_str("            prop_assert!(true); // Placeholder\n");
                } else {
                    code.push_str("            // Add your property-based assertions here\n");
                    code.push_str("            prop_assert!(true); // Placeholder\n");
                }
            }

            code.push_str("        }\n");
            code.push_str("    }\n");
        } else {
            code.push_str(&format!("    // Property-based testing not applicable for this function\n"));
            code.push_str(&format!("    // Function doesn't have numeric parameters\n"));
        }

        code.push_str("}\n");

        code
    }

    fn generate_mock_value(&self, ty: &str) -> String {
        let ty_lower = ty.to_lowercase();
        if ty_lower.contains("string") {
            "\"test_value\".to_string()".to_string()
        } else if ty_lower.contains("i32") {
            "42".to_string()
        } else if ty_lower.contains("i64") {
            "42i64".to_string()
        } else if ty_lower.contains("u32") {
            "42u32".to_string()
        } else if ty_lower.contains("u64") {
            "42u64".to_string()
        } else if ty_lower.contains("f32") {
            "3.14f32".to_string()
        } else if ty_lower.contains("f64") {
            "3.14".to_string()
        } else if ty_lower.contains("bool") {
            "true".to_string()
        } else if ty_lower.contains("vec") {
            "vec![1, 2, 3]".to_string()
        } else if ty_lower.contains("hashmap") {
            "HashMap::new()".to_string()
        } else if ty_lower.contains("option") {
            "Some(\"test\".to_string())".to_string()
        } else if ty_lower.contains("result") {
            "Ok(\"success\".to_string())".to_string()
        } else {
            format!("{}::default()", ty)
        }
    }

    fn generate_integration_mock_value(&self, ty: &str) -> String {
        let ty_lower = ty.to_lowercase();
        if ty_lower.contains("string") {
            "\"integration_test_value\".to_string()".to_string()
        } else if ty_lower.contains("i32") {
            "100".to_string()
        } else if ty_lower.contains("i64") {
            "1000i64".to_string()
        } else if ty_lower.contains("u32") {
            "100u32".to_string()
        } else if ty_lower.contains("u64") {
            "1000u64".to_string()
        } else if ty_lower.contains("f32") {
            "1.414f32".to_string()
        } else if ty_lower.contains("f64") {
            "2.718".to_string()
        } else if ty_lower.contains("bool") {
            "false".to_string() // Test different path
        } else if ty_lower.contains("vec") {
            "vec![10, 20, 30, 40, 50]".to_string()
        } else if ty_lower.contains("hashmap") {
            "{\n        let mut map = HashMap::new();\n        map.insert(\"key1\".to_string(), \"value1\".to_string());\n        map\n    }".to_string()
        } else if ty_lower.contains("option") {
            "None".to_string() // Test None path
        } else if ty_lower.contains("result") {
            "Err(\"integration test error\".to_string())".to_string()
        } else {
            format!("{}::default()", ty)
        }
    }

    fn generate_proptest_params(&self, params: &[ParamInfo]) -> String {
        params.iter()
            .filter_map(|p| {
                let ty_lower = p.ty.to_lowercase();
                if ty_lower.contains("i32") {
                    Some(format!("{} in 0..1000i32", p.name))
                } else if ty_lower.contains("i64") {
                    Some(format!("{} in 0..10000i64", p.name))
                } else if ty_lower.contains("u32") {
                    Some(format!("{} in 0..1000u32", p.name))
                } else if ty_lower.contains("u64") {
                    Some(format!("{} in 0..10000u64", p.name))
                } else if ty_lower.contains("f32") {
                    Some(format!("{} in 0.0..1000.0f32", p.name))
                } else if ty_lower.contains("f64") {
                    Some(format!("{} in 0.0..1000.0", p.name))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn snake_to_pascal(&self, snake_case: &str) -> String {
        snake_case.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars.as_str().chars()).collect(),
                }
            })
            .collect()
    }

    fn display_generated_tests(&self, tests: &[GeneratedTest], format: OutputFormat) {
        match format {
            OutputFormat::Json => {
                println!("{}", serde_json::to_string_pretty(tests).unwrap());
            }
            OutputFormat::Table => {
                println!("{:<25} {:<30}", "Function", "Generated Test");
                println!("{}", "─".repeat(55));

                for test in tests {
                    println!("{:<25} {:<30}", test.function_name, test.test_name);
                }
            }
            OutputFormat::Human => {
                println!("{}", "🧪 Generated Tests".bold().blue());
                println!("{}", "═".repeat(50).blue());

                for test in tests {
                    println!("📝 {} -> {}", test.function_name.green(), test.test_name.cyan());
                    println!("```rust");
                    println!("{}", test.test_code.trim());
                    println!("```");
                    println!();
                }

                println!("💡 {} tests generated successfully!", tests.len());
                println!("📍 Add these tests to your test module or create a new test file");
            }
        }
    }
}

struct FunctionVisitor {
    functions: Vec<FunctionInfo>,
}

impl FunctionVisitor {
    fn new() -> Self {
        Self {
            functions: Vec::new(),
        }
    }
}

impl<'ast> Visit<'ast> for FunctionVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let fn_name = node.sig.ident.to_string();
        let is_async = node.sig.asyncness.is_some();

        let params = node.sig.inputs.iter().filter_map(|arg| {
            match arg {
                FnArg::Receiver(_) => None, // Skip self parameter
                FnArg::Typed(pat_type) => {
                    if let Pat::Ident(pat_ident) = &*pat_type.pat {
                        let param_name = pat_ident.ident.to_string();
                        let param_type = pat_type.ty.to_token_stream().to_string();
                        let is_reference = param_type.contains('&');

                        Some(ParamInfo {
                            name: param_name,
                            ty: param_type.replace('&', "").replace("mut", "").trim().to_string(),
                            is_reference,
                        })
                    } else {
                        None
                    }
                }
            }
        }).collect();

        let return_type = match &node.sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, ty) => Some(ty.to_token_stream().to_string()),
        };

        let visibility = match &node.vis {
            syn::Visibility::Public(_) => "public".to_string(),
            _ => "private".to_string(),
        };

        self.functions.push(FunctionInfo {
            name: fn_name,
            params,
            return_type,
            is_async,
            visibility,
        });
    }
}

impl Tool for TestGenTool {
    fn name(&self) -> &'static str {
        "test-gen"
    }

    fn description(&self) -> &'static str {
        "Generate test boilerplate from functions"
    }

    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about("Parse Rust AST to find functions and generate test templates with edge cases")
            .args(&[
                Arg::new("file")
                    .long("file")
                    .short('f')
                    .help("Path to Rust source file")
                    .required(true),
                Arg::new("module")
                    .long("module")
                    .short('m')
                    .help("Module name for generated tests"),
                Arg::new("type")
                    .long("type")
                    .short('t')
                    .help("Test type to generate")
                    .value_parser(["unit", "integration", "property"])
                    .default_value("unit"),
                Arg::new("output")
                    .long("output")
                    .short('o')
                    .help("Output file path for generated tests"),
            ])
            .args(&common_options())
    }

    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let file_path = matches.get_one::<String>("file").unwrap();
        let test_type = matches.get_one::<String>("type").unwrap();
        let module_name = matches.get_one::<String>("module");
        let output_file = matches.get_one::<String>("output");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(ToolError::ExecutionFailed(format!("File not found: {}", file_path)));
        }

        println!("🧪 {} - Generating tests", "CargoMate TestGen".bold().blue());
        println!("   File: {} | Type: {} | Format: {:?}", file_path, test_type, output_format);

        // Parse the Rust file
        let functions = self.parse_rust_file(path)?;

        if functions.is_empty() {
            println!("⚠️  No functions found in {}", file_path);
            return Ok(());
        }

        if verbose {
            println!("📊 Found {} functions", functions.len());
        }

        // Generate tests for each function
        let mut generated_tests = Vec::new();
        for func in &functions {
            if verbose {
                println!("   Generating {} test for: {}", test_type, func.name);
            }
            let test = self.generate_test_for_function(func, test_type);
            generated_tests.push(test);
        }

        // Display or save results
        if let Some(output_path) = output_file {
            let mut output_content = String::new();

            if let Some(mod_name) = module_name {
                output_content.push_str(&format!("#[cfg(test)]\nmod {} {{\n    use super::*;\n\n", mod_name));
            } else {
                output_content.push_str("#[cfg(test)]\nmod generated_tests {\n    use super::*;\n\n");
            }

            for test in &generated_tests {
                output_content.push_str(&test.test_code);
                output_content.push_str("\n");
            }

            output_content.push_str("}\n");

            fs::write(output_path, output_content)
                .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write output file: {}", e)))?;

            println!("💾 Tests saved to: {}", output_path.cyan());
        } else {
            self.display_generated_tests(&generated_tests, output_format);
        }

        Ok(())
    }
}

impl Default for TestGenTool {
    fn default() -> Self {
        Self::new()
    }
}
