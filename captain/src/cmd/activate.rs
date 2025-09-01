use anyhow::Result;
use std::fs;
use colored::Colorize;
use crate::captain::shell_integration::{detect_shell, get_rc_file};
pub fn handle_activate() -> Result<()> {
    println!("⚡ Activating Cargo Mate shell integration...");
    let shell = detect_shell();
    let rc_file = get_rc_file(&shell)?;
    if !rc_file.exists() {
        println!("❌ No shell configuration file found: {}", rc_file.display());
        println!("💡 Run 'cm init' first to set up shell integration");
        return Ok(());
    }
    let content = fs::read_to_string(&rc_file)?;
    if !content.contains("# === Cargo Mate") {
        println!("❌ Cargo Mate integration not found in {}", rc_file.display());
        println!("💡 Run 'cm init' first to set up shell integration");
        return Ok(());
    }
    println!("🔄 Sourcing {}", rc_file.display().to_string().cyan());
    let output = std::process::Command::new(&shell)
        .arg("-c")
        .arg(format!("source {} && env", rc_file.display()))
        .output()?;
    if output.status.success() {
        println!("✅ Shell integration activated successfully!");
        println!();
        println!("🚢 {}", "You can now use:".yellow());
        println!("   {} - cargo commands go through cargo-mate", "cargo".cyan());
        println!("   {} - direct cargo-mate access", "cm".cyan());
        println!("   {} - quick alias", "cg".cyan());
        println!();
        println!("🎯 {}", "Try it:".green());
        println!("   cargo --version");
        println!("   cm --help");
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        println!("❌ Failed to activate integration: {}", error);
        println!(
            "💡 You can manually run: {}", format!("source {}", rc_file.display())
            .green()
        );
    }
    Ok(())
}
pub fn handle_install() -> Result<()> {
    println!("📦 Installing Cargo Mate shell integration...");
    let shell = detect_shell();
    let rc_file = get_rc_file(&shell)?;
    if rc_file.exists() {
        let content = std::fs::read_to_string(&rc_file)?;
        if content.contains("# === Cargo Mate") {
            println!("✅ Shell integration already installed");
            return Ok(());
        }
    }
    let integration_script = format!(
        r#"
# === Cargo Mate ===
export PATH="$HOME/.shipwreck/bin:$PATH"
alias cargo="cargo-mate"
alias cg="cargo-mate"
# === End Cargo Mate ===
"#
    );
    let mut content = if rc_file.exists() {
        std::fs::read_to_string(&rc_file)?
    } else {
        String::new()
    };
    content.push_str(&integration_script);
    std::fs::write(&rc_file, content)?;
    println!("✅ Shell integration installed to {}", rc_file.display());
    println!("💡 Run 'cm activate' to activate it in this session");
    println!("💡 Or restart your shell to use it automatically");
    Ok(())
}