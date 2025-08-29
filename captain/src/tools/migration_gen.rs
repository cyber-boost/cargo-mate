use super::{Tool, Result, ToolError, common_options, parse_output_format, OutputFormat};
use clap::{Arg, ArgMatches, Command};
use colored::*;
use std::path::Path;
use std::fs;
use std::collections::HashMap;
use syn::{parse_file, Item, ItemStruct, Fields, Field, Type, Path as SynPath};
use quote::ToTokens;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone)]
pub struct MigrationGenTool;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StructDefinition {
    name: String,
    fields: Vec<FieldDefinition>,
    file_path: String,
    line_number: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FieldDefinition {
    name: String,
    ty: String,
    is_optional: bool,
    attributes: Vec<String>,
    comment: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationPlan {
    up_sql: String,
    down_sql: String,
    description: String,
    timestamp: String,
    table_name: String,
    changes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MigrationReport {
    migrations_generated: usize,
    tables_created: usize,
    files_created: Vec<String>,
    total_changes: usize,
    migration_plan: Vec<MigrationPlan>,
}
impl MigrationGenTool {
    pub fn new() -> Self {
        Self
    }
    fn parse_rust_structs(&self, file_path: &str) -> Result<Vec<StructDefinition>> {
        let content = fs::read_to_string(file_path)?;
        let syntax = parse_file(&content)?;
        let mut structs = Vec::new();
        for (i, item) in syntax.items.iter().enumerate() {
            if let Item::Struct(struct_def) = item {
                let struct_def = self.parse_struct_definition(struct_def, file_path, i)?;
                structs.push(struct_def);
            }
        }
        Ok(structs)
    }
    fn parse_struct_definition(
        &self,
        struct_def: &ItemStruct,
        file_path: &str,
        line_number: usize,
    ) -> Result<StructDefinition> {
        let name = struct_def.ident.to_string();
        let mut fields = Vec::new();
        if let Fields::Named(named_fields) = &struct_def.fields {
            for field in &named_fields.named {
                let field_def = self.parse_field_definition(field)?;
                fields.push(field_def);
            }
        }
        Ok(StructDefinition {
            name,
            fields,
            file_path: file_path.to_string(),
            line_number,
        })
    }
    fn parse_field_definition(&self, field: &Field) -> Result<FieldDefinition> {
        let name = field
            .ident
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("Field without name".to_string()))?
            .to_string();
        let ty = self.type_to_sql_type(&field.ty)?;
        let is_optional = self.is_optional_type(&field.ty);
        let mut attributes = Vec::new();
        for attr in &field.attrs {
            let attr_str = attr.to_token_stream().to_string();
            attributes.push(attr_str);
        }
        let comment = field
            .attrs
            .iter()
            .find(|attr| attr.path().segments.last().unwrap().ident == "doc")
            .and_then(|attr| {
                if let Ok(syn::Meta::NameValue(meta)) = attr.parse_args::<syn::Meta>() {
                    Some("doc_comment_placeholder".to_string())
                } else {
                    None
                }
            });
        Ok(FieldDefinition {
            name,
            ty,
            is_optional,
            attributes,
            comment,
        })
    }
    fn type_to_sql_type(&self, ty: &Type) -> Result<String> {
        match ty {
            Type::Path(type_path) => {
                if let Some(segment) = type_path.path.segments.last() {
                    match segment.ident.to_string().as_str() {
                        "String" => Ok("VARCHAR(255)".to_string()),
                        "i32" | "i64" | "isize" => Ok("INTEGER".to_string()),
                        "u32" | "u64" | "usize" => Ok("BIGINT UNSIGNED".to_string()),
                        "f32" | "f64" => Ok("DECIMAL(10,2)".to_string()),
                        "bool" => Ok("BOOLEAN".to_string()),
                        "NaiveDateTime" | "DateTime" => Ok("TIMESTAMP".to_string()),
                        "NaiveDate" => Ok("DATE".to_string()),
                        "Uuid" => Ok("UUID".to_string()),
                        "Vec" => Ok("JSON".to_string()),
                        "HashMap" | "BTreeMap" => Ok("JSON".to_string()),
                        "Option" => {
                            if let syn::PathArguments::AngleBracketed(args) = &segment
                                .arguments
                            {
                                if let Some(syn::GenericArgument::Type(inner_ty)) = args
                                    .args
                                    .first()
                                {
                                    let inner_sql = self.type_to_sql_type(inner_ty)?;
                                    Ok(inner_sql)
                                } else {
                                    Ok("VARCHAR(255)".to_string())
                                }
                            } else {
                                Ok("VARCHAR(255)".to_string())
                            }
                        }
                        _ => Ok("VARCHAR(255)".to_string()),
                    }
                } else {
                    Ok("VARCHAR(255)".to_string())
                }
            }
            _ => Ok("VARCHAR(255)".to_string()),
        }
    }
    fn is_optional_type(&self, ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                segment.ident == "Option"
            } else {
                false
            }
        } else {
            false
        }
    }
    fn generate_create_table_sql(&self, struct_def: &StructDefinition) -> String {
        let table_name = self.struct_name_to_table_name(&struct_def.name);
        let mut sql = format!("CREATE TABLE {} (\n", table_name);
        sql.push_str("    id SERIAL PRIMARY KEY,\n");
        for (i, field) in struct_def.fields.iter().enumerate() {
            let column_name = self.field_name_to_column_name(&field.name);
            let sql_type = &field.ty;
            let nullable = if field.is_optional { "" } else { " NOT NULL" };
            let comma = if i < struct_def.fields.len() - 1 { "," } else { "" };
            sql.push_str(&format!("    {} {}{}", column_name, sql_type, nullable));
            if let Some(comment) = &field.comment {
                sql.push_str(&format!(" -- {}", comment));
            }
            sql.push_str(&format!("{}\n", comma));
        }
        sql.push_str(");\n");
        sql
    }
    fn generate_migration_sql(&self, struct_def: &StructDefinition) -> MigrationPlan {
        let table_name = self.struct_name_to_table_name(&struct_def.name);
        let up_sql = self.generate_create_table_sql(struct_def);
        let down_sql = format!("DROP TABLE IF EXISTS {};\n", table_name);
        let description = format!(
            "Create {} table for {} struct", table_name, struct_def.name
        );
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let changes = vec![
            format!("Create table with {} columns", struct_def.fields.len())
        ];
        MigrationPlan {
            up_sql,
            down_sql,
            description,
            timestamp,
            table_name,
            changes,
        }
    }
    fn struct_name_to_table_name(&self, struct_name: &str) -> String {
        let mut table_name = String::new();
        for (i, c) in struct_name.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                table_name.push('_');
            }
            table_name.push(c.to_lowercase().next().unwrap());
        }
        format!("{}_table", table_name)
    }
    fn field_name_to_column_name(&self, field_name: &str) -> String {
        let mut column_name = String::new();
        for (i, c) in field_name.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                column_name.push('_');
            }
            column_name.push(c.to_lowercase().next().unwrap());
        }
        column_name
    }
    fn generate_migration_file(
        &self,
        plan: &MigrationPlan,
        output_dir: &str,
    ) -> Result<String> {
        let file_name = format!("{}_{}.sql", plan.timestamp, plan.table_name);
        let file_path = Path::new(output_dir).join(&file_name);
        fs::create_dir_all(output_dir)?;
        let mut content = format!("-- Migration: {}\n", plan.description);
        content.push_str(&format!("-- Generated at: {}\n", plan.timestamp));
        content.push_str("-- +migrate Up\n");
        content.push_str(&plan.up_sql);
        content.push_str("-- +migrate Down\n");
        content.push_str(&plan.down_sql);
        fs::write(&file_path, content)?;
        Ok(file_path.to_string_lossy().to_string())
    }
    fn find_existing_migration(
        &self,
        table_name: &str,
        migrations_dir: &str,
    ) -> Option<String> {
        if let Ok(entries) = fs::read_dir(migrations_dir) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.contains(&table_name) && file_name.ends_with(".sql") {
                        return Some(entry.path().to_string_lossy().to_string());
                    }
                }
            }
        }
        None
    }
    fn compare_struct_with_existing(
        &self,
        new_struct: &StructDefinition,
        existing_migration: &str,
    ) -> Result<Vec<String>> {
        let existing_content = fs::read_to_string(existing_migration)?;
        let mut changes = Vec::new();
        let existing_columns = self.extract_columns_from_sql(&existing_content)?;
        for field in &new_struct.fields {
            let column_name = self.field_name_to_column_name(&field.name);
            if let Some(existing_col) = existing_columns.get(&column_name) {
                if existing_col != &field.ty {
                    changes
                        .push(
                            format!(
                                "ALTER COLUMN {} TYPE {} -> {}", column_name, existing_col,
                                field.ty
                            ),
                        );
                }
            } else {
                changes.push(format!("ADD COLUMN {} {}", column_name, field.ty));
            }
        }
        Ok(changes)
    }
    fn extract_columns_from_sql(
        &self,
        sql_content: &str,
    ) -> Result<HashMap<String, String>> {
        let mut columns = HashMap::new();
        for line in sql_content.lines() {
            let line = line.trim();
            if line.starts_with("CREATE TABLE") || line.is_empty()
                || line.starts_with("--")
            {
                continue;
            }
            if line.starts_with(");") {
                break;
            }
            if let Some((column_name, column_type)) = self.parse_column_definition(line)
            {
                columns.insert(column_name, column_type);
            }
        }
        Ok(columns)
    }
    fn parse_column_definition(&self, line: &str) -> Option<(String, String)> {
        let line = line.trim().trim_end_matches(',');
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let column_name = parts[0].to_string();
            let column_type = parts[1..].join(" ");
            Some((column_name, column_type))
        } else {
            None
        }
    }
    fn generate_alter_migration(
        &self,
        struct_def: &StructDefinition,
        changes: &[String],
        timestamp: &str,
    ) -> MigrationPlan {
        let table_name = self.struct_name_to_table_name(&struct_def.name);
        let mut up_sql = format!("-- Alter {} table\n", table_name);
        let mut down_sql = format!("-- Revert {} table changes\n", table_name);
        for change in changes {
            if change.starts_with("ADD COLUMN") {
                up_sql.push_str(&format!("ALTER TABLE {} {};\n", table_name, change));
                down_sql
                    .push_str(
                        &format!("-- Would need to drop column for full revert\n"),
                    );
            } else if change.starts_with("ALTER COLUMN") {
                up_sql.push_str(&format!("-- {} (manual review required)\n", change));
                down_sql
                    .push_str(
                        &format!("-- Revert {} (manual review required)\n", change),
                    );
            }
        }
        let description = format!(
            "Alter {} table - {} changes", table_name, changes.len()
        );
        MigrationPlan {
            up_sql,
            down_sql,
            description,
            timestamp: timestamp.to_string(),
            table_name,
            changes: changes.to_vec(),
        }
    }
    fn display_report(
        &self,
        report: &MigrationReport,
        output_format: OutputFormat,
        verbose: bool,
    ) {
        match output_format {
            OutputFormat::Human => {
                println!(
                    "\n🗃️  {} - SQL Migration Generation Report",
                    "CargoMate MigrationGen".bold().blue()
                );
                println!("{}", "═".repeat(60).blue());
                println!("\n📊 Summary:");
                println!("  • Migrations Generated: {}", report.migrations_generated);
                println!("  • Tables Created: {}", report.tables_created);
                println!("  • Total Changes: {}", report.total_changes);
                if !report.files_created.is_empty() {
                    println!("\n📁 Files Created:");
                    for file in &report.files_created {
                        println!("  • {}", file.green());
                    }
                }
                if verbose {
                    println!("\n🔧 Migration Plans:");
                    for plan in &report.migration_plan {
                        println!(
                            "  \n📋 {} ({})", plan.description.cyan(), plan.timestamp
                        );
                        println!("    Table: {}", plan.table_name.yellow());
                        if !plan.changes.is_empty() {
                            println!("    Changes:");
                            for change in &plan.changes {
                                println!("      • {}", change);
                            }
                        }
                        println!("    Up SQL Preview:");
                        for line in plan.up_sql.lines().take(5) {
                            println!("      {}", line.dimmed());
                        }
                        if plan.up_sql.lines().count() > 5 {
                            println!(
                                "      ... ({} more lines)", plan.up_sql.lines().count() - 5
                            );
                        }
                    }
                }
                println!("\n💡 Next Steps:");
                println!("  1. Review generated migration files");
                println!("  2. Test migrations on a development database");
                println!("  3. Run migrations in your CI/CD pipeline");
                println!("  4. Create database backups before running migrations");
                println!("\n🔧 Common Migration Commands:");
                println!("  • PostgreSQL: psql -f migration.sql");
                println!("  • MySQL: mysql < migration.sql");
                println!("  • SQLite: sqlite3 database.db < migration.sql");
                println!("  • Diesel: diesel migration run");
                println!("  • SeaORM: sea-orm-cli migrate up");
            }
            OutputFormat::Json => {
                let json = serde_json::to_string_pretty(report)
                    .unwrap_or_else(|_| "{}".to_string());
                println!("{}", json);
            }
            OutputFormat::Table => {
                println!(
                    "{:<25} {:<20} {:<15} {:<10}", "Table", "Description", "Changes",
                    "Timestamp"
                );
                println!("{}", "─".repeat(75));
                for plan in &report.migration_plan {
                    println!(
                        "{:<25} {:<20} {:<15} {:<10}", plan.table_name, plan.description
                        .chars().take(18).collect::< String > (), plan.changes.len()
                        .to_string(), plan.timestamp.chars().take(8).collect::< String >
                        ()
                    );
                }
            }
        }
    }
}
impl Tool for MigrationGenTool {
    fn name(&self) -> &'static str {
        "migration-gen"
    }
    fn description(&self) -> &'static str {
        "Generate SQL migrations from struct changes"
    }
    fn command(&self) -> Command {
        Command::new(self.name())
            .about(self.description())
            .long_about(
                "Generate SQL migration scripts from Rust struct definitions. \
                        Automatically detects changes between struct versions and creates \
                        appropriate ALTER/CREATE TABLE statements.

EXAMPLES:
    cm tool migration-gen --input src/models.rs --output migrations/
    cm tool migration-gen --input src/user.rs --database postgres
    cm tool migration-gen --existing-schema schema.sql --diff",
            )
            .args(
                &[
                    Arg::new("input")
                        .long("input")
                        .short('i')
                        .help("Input Rust file containing struct definitions")
                        .required(true),
                    Arg::new("output")
                        .long("output")
                        .short('o')
                        .help("Output directory for migration files")
                        .default_value("migrations/"),
                    Arg::new("database")
                        .long("database")
                        .short('d')
                        .help("Target database (postgres, mysql, sqlite, mssql)")
                        .default_value("postgres"),
                    Arg::new("existing-schema")
                        .long("existing-schema")
                        .help("Path to existing schema file for diff"),
                    Arg::new("diff")
                        .long("diff")
                        .help("Generate diff migrations against existing schema")
                        .action(clap::ArgAction::SetTrue),
                    Arg::new("migrations-dir")
                        .long("migrations-dir")
                        .help("Directory containing existing migrations")
                        .default_value("migrations/"),
                    Arg::new("framework")
                        .long("framework")
                        .short('f')
                        .help("Migration framework (diesel, seaorm, raw)")
                        .default_value("raw"),
                    Arg::new("dry-run")
                        .long("dry-run")
                        .help("Show what would be generated without creating files")
                        .action(clap::ArgAction::SetTrue),
                ],
            )
            .args(&common_options())
    }
    fn execute(&self, matches: &ArgMatches) -> Result<()> {
        let input_file = matches.get_one::<String>("input").unwrap();
        let output_dir = matches.get_one::<String>("output").unwrap();
        let database = matches.get_one::<String>("database").unwrap();
        let existing_schema = matches.get_one::<String>("existing-schema");
        let diff = matches.get_flag("diff");
        let migrations_dir = matches.get_one::<String>("migrations-dir").unwrap();
        let framework = matches.get_one::<String>("framework").unwrap();
        let dry_run = matches.get_flag("dry-run");
        let output_format = parse_output_format(matches);
        let verbose = matches.get_flag("verbose");
        println!(
            "🗃️  {} - Generating SQL Migrations", "CargoMate MigrationGen".bold()
            .blue()
        );
        if !Path::new(input_file).exists() {
            return Err(
                ToolError::InvalidArguments(
                    format!("Input file {} not found", input_file),
                ),
            );
        }
        let structs = self.parse_rust_structs(input_file)?;
        if structs.is_empty() {
            return Err(
                ToolError::ExecutionFailed(
                    "No struct definitions found in input file".to_string(),
                ),
            );
        }
        if verbose {
            println!("\n📋 Found {} struct(s):", structs.len());
            for struct_def in &structs {
                println!(
                    "  • {} - {} fields", struct_def.name.green(), struct_def.fields
                    .len()
                );
            }
        }
        let mut migration_plans = Vec::new();
        let mut files_created = Vec::new();
        for struct_def in &structs {
            if diff {
                let table_name = self.struct_name_to_table_name(&struct_def.name);
                if let Some(existing_migration) = self
                    .find_existing_migration(&table_name, migrations_dir)
                {
                    if let Ok(changes) = self
                        .compare_struct_with_existing(struct_def, &existing_migration)
                    {
                        if !changes.is_empty() {
                            let timestamp = chrono::Utc::now()
                                .format("%Y%m%d_%H%M%S")
                                .to_string();
                            let plan = self
                                .generate_alter_migration(struct_def, &changes, &timestamp);
                            migration_plans.push(plan);
                        }
                    }
                } else {
                    let plan = self.generate_migration_sql(struct_def);
                    migration_plans.push(plan);
                }
            } else {
                let plan = self.generate_migration_sql(struct_def);
                migration_plans.push(plan);
            }
        }
        if migration_plans.is_empty() {
            println!("{}", "No migrations needed - no changes detected".yellow());
            return Ok(());
        }
        for plan in &migration_plans {
            if !dry_run {
                match self.generate_migration_file(plan, output_dir) {
                    Ok(file_path) => {
                        files_created.push(file_path);
                        if verbose {
                            println!("✅ Generated migration: {}", plan.table_name);
                        }
                    }
                    Err(e) => {
                        println!(
                            "❌ Failed to generate migration for {}: {}", plan
                            .table_name, e
                        );
                    }
                }
            }
        }
        let report = MigrationReport {
            migrations_generated: migration_plans.len(),
            tables_created: migration_plans
                .iter()
                .filter(|p| p.up_sql.contains("CREATE TABLE"))
                .count(),
            files_created: files_created.clone(),
            total_changes: migration_plans.iter().map(|p| p.changes.len()).sum(),
            migration_plan: migration_plans,
        };
        self.display_report(&report, output_format, verbose);
        if dry_run {
            println!("\n🔍 Dry run complete - no files were created");
        } else if !files_created.is_empty() {
            println!("\n✅ Generated {} migration file(s)", files_created.len());
        }
        Ok(())
    }
}
impl Default for MigrationGenTool {
    fn default() -> Self {
        Self::new()
    }
}