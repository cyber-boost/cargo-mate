use anyhow::{Context, Result};
use colored::*;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
#[derive(Debug, Clone)]
pub struct SmartError {
    pub code: String,
    pub message: String,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub suggestion: Option<String>,
    pub fix_command: Option<String>,
    pub explanation: Option<String>,
    pub related_docs: Vec<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct ErrorPattern {
    code: String,
    pattern: String,
    suggestion: String,
    fix_command: Option<String>,
    docs_url: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct ErrorDatabase {
    patterns: Vec<ErrorPattern>,
    common_fixes: HashMap<String, Vec<String>>,
    learning_data: HashMap<String, FixHistory>,
}
#[derive(Debug, Serialize, Deserialize)]
struct FixHistory {
    successful_fixes: Vec<String>,
    failed_attempts: Vec<String>,
    frequency: usize,
}
pub struct SmartParser {
    error_db: ErrorDatabase,
    db_file: PathBuf,
    online_search: bool,
}
impl SmartParser {
    pub fn new() -> Result<Self> {
        let db_file = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("error_db.json");
        let error_db = if db_file.exists() {
            let content = fs::read_to_string(&db_file)?;
            serde_json::from_str(&content)?
        } else {
            Self::create_default_database()
        };
        Ok(Self {
            error_db,
            db_file,
            online_search: true,
        })
    }
    fn create_default_database() -> ErrorDatabase {
        let patterns = vec![
            ErrorPattern { code : "E0308".to_string(), pattern : "mismatched types"
            .to_string(), suggestion :
            "Check the expected and found types. You may need to convert types using methods like .to_string(), .into(), or as_ref()"
            .to_string(), fix_command : Some("cargo fix --allow-dirty".to_string()),
            docs_url : Some("https://doc.rust-lang.org/error-index.html#E0308"
            .to_string()), }, ErrorPattern { code : "E0382".to_string(), pattern :
            "borrow of moved value".to_string(), suggestion :
            "The value has been moved. Consider cloning it, using a reference, or restructuring ownership"
            .to_string(), fix_command : None, docs_url :
            Some("https://doc.rust-lang.org/error-index.html#E0382".to_string()), },
            ErrorPattern { code : "E0499".to_string(), pattern :
            "cannot borrow .* as mutable more than once".to_string(), suggestion :
            "You have multiple mutable borrows. Consider using RefCell for interior mutability or restructuring your code"
            .to_string(), fix_command : None, docs_url :
            Some("https://doc.rust-lang.org/error-index.html#E0499".to_string()), },
            ErrorPattern { code : "E0277".to_string(), pattern :
            "the trait bound .* is not satisfied".to_string(), suggestion :
            "The required trait is not implemented. Check if you need to import the trait or implement it for your type"
            .to_string(), fix_command : None, docs_url :
            Some("https://doc.rust-lang.org/error-index.html#E0277".to_string()), },
            ErrorPattern { code : "E0433".to_string(), pattern : "failed to resolve"
            .to_string(), suggestion :
            "Module or item not found. Check your imports and module declarations"
            .to_string(), fix_command : Some("cargo check".to_string()), docs_url :
            Some("https://doc.rust-lang.org/error-index.html#E0433".to_string()), },
        ];
        let mut common_fixes = HashMap::new();
        common_fixes
            .insert(
                "lifetime".to_string(),
                vec![
                    "Add lifetime parameters to your struct/function".to_string(),
                    "Use 'static lifetime if the value needs to live for the entire program"
                    .to_string(), "Consider using Arc<T> or Rc<T> for shared ownership"
                    .to_string(),
                ],
            );
        common_fixes
            .insert(
                "async".to_string(),
                vec![
                    "Make sure you're using .await on async functions".to_string(),
                    "Check if your function needs to be async".to_string(),
                    "Consider using tokio::spawn or async blocks".to_string(),
                ],
            );
        common_fixes
            .insert(
                "iterator".to_string(),
                vec![
                    "Check if you need to call .collect() to consume the iterator"
                    .to_string(),
                    "Make sure you're using the right iterator method (map, filter, fold, etc.)"
                    .to_string(),
                    "Consider using .iter() or .into_iter() based on ownership needs"
                    .to_string(),
                ],
            );
        ErrorDatabase {
            patterns,
            common_fixes,
            learning_data: HashMap::new(),
        }
    }
    pub fn parse_error(&mut self, error_text: &str) -> SmartError {
        let code = self.extract_error_code(error_text);
        let (file, line, column) = self.extract_location(error_text);
        let message = self.extract_message(error_text);
        let suggestion = self.get_suggestion(&code, &message);
        let fix_command = self.get_fix_command(&code);
        let explanation = self.get_explanation(&code, &message);
        let related_docs = self.get_related_docs(&code);
        self.learn_from_error(&code, &message);
        SmartError {
            code,
            message,
            file,
            line,
            column,
            suggestion,
            fix_command,
            explanation,
            related_docs,
        }
    }
    fn extract_error_code(&self, text: &str) -> String {
        let re = Regex::new(r"error\[([A-Z0-9]+)\]").unwrap();
        re.captures(text)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    fn extract_location(&self, text: &str) -> (PathBuf, usize, usize) {
        let re = Regex::new(r"(\S+\.rs):(\d+):(\d+)").unwrap();
        if let Some(cap) = re.captures(text) {
            let file = PathBuf::from(cap.get(1).unwrap().as_str());
            let line = cap.get(2).unwrap().as_str().parse().unwrap_or(0);
            let column = cap.get(3).unwrap().as_str().parse().unwrap_or(0);
            (file, line, column)
        } else {
            (PathBuf::from("unknown"), 0, 0)
        }
    }
    fn extract_message(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        if let Some(error_line) = lines.iter().find(|l| l.contains("error")) {
            let parts: Vec<&str> = error_line.split(':').collect();
            if parts.len() > 1 {
                return parts[1..].join(":").trim().to_string();
            }
        }
        text.to_string()
    }
    fn get_suggestion(&self, code: &str, message: &str) -> Option<String> {
        for pattern in &self.error_db.patterns {
            if pattern.code == code {
                return Some(pattern.suggestion.clone());
            }
        }
        if message.contains("lifetime") {
            if let Some(fixes) = self.error_db.common_fixes.get("lifetime") {
                return Some(fixes.join("\n"));
            }
        }
        if message.contains("async") || message.contains("await") {
            if let Some(fixes) = self.error_db.common_fixes.get("async") {
                return Some(fixes.join("\n"));
            }
        }
        if self.online_search {
            self.search_online_suggestion(code, message).ok()
        } else {
            None
        }
    }
    fn get_fix_command(&self, code: &str) -> Option<String> {
        self.error_db
            .patterns
            .iter()
            .find(|p| p.code == code)
            .and_then(|p| p.fix_command.clone())
    }
    fn get_explanation(&self, code: &str, message: &str) -> Option<String> {
        let base_explanation = match code {
            "E0308" => {
                "Type mismatch occurs when Rust expects one type but finds another."
            }
            "E0382" => {
                "Ownership has been transferred. In Rust, each value has a single owner."
            }
            "E0499" => {
                "Rust's borrowing rules prevent multiple mutable references to prevent data races."
            }
            "E0277" => {
                "A trait bound was not satisfied. The type doesn't implement the required trait."
            }
            "E0433" => "Failed to resolve a path to a module, type, or function.",
            _ => return None,
        };
        Some(format!("{}\n\nContext: {}", base_explanation, message))
    }
    fn get_related_docs(&self, code: &str) -> Vec<String> {
        let mut docs = vec![
            format!("https://doc.rust-lang.org/error-index.html#{}", code),
        ];
        if let Some(pattern) = self.error_db.patterns.iter().find(|p| p.code == code) {
            if let Some(ref url) = pattern.docs_url {
                docs.push(url.clone());
            }
        }
        docs.push("https://doc.rust-lang.org/book/".to_string());
        docs
    }
    fn search_online_suggestion(&self, code: &str, message: &str) -> Result<String> {
        Ok(
            format!(
                "💡 Search for solutions:\n  - Stack Overflow: rust {} {}\n  - Rust Forum: https://users.rust-lang.org/",
                code, message.split_whitespace().take(5).collect::< Vec < _ >> ()
                .join(" ")
            ),
        )
    }
    fn learn_from_error(&mut self, code: &str, _message: &str) {
        let entry = self
            .error_db
            .learning_data
            .entry(code.to_string())
            .or_insert(FixHistory {
                successful_fixes: Vec::new(),
                failed_attempts: Vec::new(),
                frequency: 0,
            });
        entry.frequency += 1;
        let _ = self.save_database();
    }
    pub fn record_fix(&mut self, code: &str, fix: &str, successful: bool) -> Result<()> {
        let entry = self
            .error_db
            .learning_data
            .entry(code.to_string())
            .or_insert(FixHistory {
                successful_fixes: Vec::new(),
                failed_attempts: Vec::new(),
                frequency: 0,
            });
        if successful {
            entry.successful_fixes.push(fix.to_string());
        } else {
            entry.failed_attempts.push(fix.to_string());
        }
        self.save_database()?;
        Ok(())
    }
    pub fn display_smart_error(&self, error: &SmartError) {
        println!(
            "{}", format!("═══ Error {} ═══", error.code) .red().bold()
        );
        println!(
            "📁 {}:{}:{}", error.file.display(), error.line.to_string().yellow(), error
            .column
        );
        println!("📝 {}", error.message.white());
        if let Some(ref suggestion) = error.suggestion {
            println!("\n💡 {}", "Suggestion:".green().bold());
            for line in suggestion.lines() {
                println!("   {}", line);
            }
        }
        if let Some(ref cmd) = error.fix_command {
            println!("\n🔧 {}", "Quick fix:".cyan().bold());
            println!("   {}", cmd.cyan());
        }
        if let Some(ref explanation) = error.explanation {
            println!("\n📖 {}", "Explanation:".blue().bold());
            for line in explanation.lines() {
                println!("   {}", line.dimmed());
            }
        }
        if !error.related_docs.is_empty() {
            println!("\n📚 {}", "Related Documentation:".magenta().bold());
            for doc in &error.related_docs {
                println!("   • {}", doc.underline());
            }
        }
        println!("{}", "═".repeat(50).red());
    }
    pub fn suggest_learning_path(&self, errors: &[SmartError]) {
        let mut error_categories: HashMap<String, usize> = HashMap::new();
        for error in errors {
            let category = self.categorize_error(&error.code);
            *error_categories.entry(category).or_insert(0) += 1;
        }
        if error_categories.is_empty() {
            return;
        }
        println!("{}", "📚 Learning Path Recommendation".green().bold());
        println!("Based on your errors, consider studying:");
        let mut categories: Vec<_> = error_categories.into_iter().collect();
        categories.sort_by(|a, b| b.1.cmp(&a.1));
        for (category, count) in categories.iter().take(3) {
            let resources = self.get_learning_resources(category);
            println!("\n{} ({} errors)", category.cyan(), count);
            for resource in resources {
                println!("  • {}", resource);
            }
        }
    }
    fn categorize_error(&self, code: &str) -> String {
        match code {
            c if c.starts_with("E03") => "Ownership & Borrowing".to_string(),
            c if c.starts_with("E04") => "Pattern Matching".to_string(),
            c if c.starts_with("E05") => "Traits & Generics".to_string(),
            c if c.starts_with("E06") => "Modules & Visibility".to_string(),
            c if c.starts_with("E07") => "Async & Concurrency".to_string(),
            _ => "General Rust".to_string(),
        }
    }
    fn get_learning_resources(&self, category: &str) -> Vec<String> {
        match category {
            "Ownership & Borrowing" => {
                vec![
                    "The Rust Book - Chapter 4: Understanding Ownership".to_string(),
                    "Rust by Example - Ownership section".to_string(),
                    "Video: 'Rust Ownership Explained' by No Boilerplate".to_string(),
                ]
            }
            "Traits & Generics" => {
                vec![
                    "The Rust Book - Chapter 10: Generic Types, Traits".to_string(),
                    "Rust by Example - Traits section".to_string(),
                    "Blog: 'Rust Traits: A Deep Dive'".to_string(),
                ]
            }
            _ => {
                vec![
                    "The Rust Programming Language Book".to_string(), "Rust by Example"
                    .to_string(), "Rustlings exercises".to_string(),
                ]
            }
        }
    }
    fn save_database(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.error_db)?;
        fs::write(&self.db_file, json)?;
        Ok(())
    }
}