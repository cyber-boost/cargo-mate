use anyhow::Result;
use std::fs;
use colored::Colorize;
use crate::captain::shell_integration::ShellIntegration;
pub fn handle_activate() -> Result<()> {
    println!("⚡ Activating Cargo Mate shell integration...");
    let shell = ShellIntegration::detect_shell()?;
    let rc_file = ShellIntegration::get_rc_file(&shell)?;
    if !rc_file.exists() {
        handle_install()?;
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
        println!();
        println!("🚢 {}", "You can now use:".yellow());
        println!("   {} - cargo commands go through cm", "cargo".cyan());
        println!("   {} - direct cm access", "cm".cyan());
        println!("   {} - quick alias", "cg".cyan());
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
    let shell = ShellIntegration::detect_shell()?;
    let rc_file = ShellIntegration::get_rc_file(&shell)?;
    if rc_file.exists() {
        let content = std::fs::read_to_string(&rc_file)?;
        if content.contains("# === Cargo Mate") {
            return Ok(());
        }
    }
    let integration_script = format!(
        r#"
# === Cargo Mate ===
# Add ~/.shipwreck/bin to PATH for cargo-mate commands
if [[ ":$PATH:" != *":$HOME/.shipwreck/bin:"* ]]; then
    export PATH="$HOME/.shipwreck/bin:$PATH"
fi

# Function to check if cm command exists (checks multiple locations)
cm_exists() {{
    # Try multiple ways to find cm
    if command -v cm &> /dev/null; then
        return 0
    elif [ -x "$HOME/.shipwreck/bin/cm" ]; then
        return 0
    elif [ -x "$HOME/.cargo/bin/cm" ]; then
        return 0
    elif [ -f "$HOME/.shipwreck/bin/cm" ]; then
        return 0
    elif [ -f "$HOME/.cargo/bin/cm" ]; then
        return 0
    fi
    return 1
}}

# Function to find cm binary path
find_cm_binary() {{
    if command -v cm &> /dev/null; then
        command -v cm
    elif [ -x "$HOME/.shipwreck/bin/cm" ]; then
        echo "$HOME/.shipwreck/bin/cm"
    elif [ -x "$HOME/.cargo/bin/cm" ]; then
        echo "$HOME/.cargo/bin/cm"
    else
        echo "cm"
    fi
}}

# Function to intercept cargo commands
cargo() {{
    if cm_exists; then
        # Use the found cm binary
        CM_BINARY="$(find_cm_binary)"
        $CM_BINARY exec "$@"
    else
        command cargo "$@"
    fi
}}

# Alias for quick access
alias cg='cm'

# Auto-complete for cm (if available)
if [ -f ~/.shipwreck/completions/cm.bash ]; then
    source ~/.shipwreck/completions/cm.bash
fi

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
    Ok(())
}