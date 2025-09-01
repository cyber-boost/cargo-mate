use anyhow::Result;
use std::path::PathBuf;
pub fn auto_install_shell_integration() -> Result<()> {
    let shell = detect_shell();
    let rc_file = get_rc_file(&shell)?;
    if rc_file.exists() {
        let content = std::fs::read_to_string(&rc_file)?;
        if content.contains("# === Cargo Mate") {
            return handle_activate();
        }
    }
    add_shell_integration(&rc_file, &shell)?;
    handle_activate()?;
    Ok(())
}
pub fn detect_shell() -> String {
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            return "zsh".to_string();
        } else if shell.contains("bash") {
            return "bash".to_string();
        } else if shell.contains("fish") {
            return "fish".to_string();
        }
    }
    "bash".to_string()
}
pub fn get_rc_file(shell: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not find home directory")?;
    let rc_file = match shell {
        "zsh" => home.join(".zshrc"),
        "bash" => {
            let bashrc = home.join(".bashrc");
            if bashrc.exists() { bashrc } else { home.join(".bash_profile") }
        }
        "fish" => home.join(".config").join("fish").join("config.fish"),
        _ => home.join(".profile"),
    };
    Ok(rc_file)
}
pub fn add_shell_integration(rc_file: &PathBuf, shell: &str) -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;
    if rc_file.exists() {
        let backup = rc_file.with_extension("bak.cargo-mate");
        std::fs::copy(rc_file, &backup)?;
        println!("📋 Backed up {} to {}", rc_file.display(), backup.display());
    }
    let integration_code = match shell {
        "fish" => {
            r#"
# === Cargo Mate (cm) Integration ===
function cargo
    cm exec $argv
end

# Note: cm binary should be in PATH
alias cg='cm'
# === End Cargo Mate Integration ===
"#
        }
        _ => {
            r#"
# === Cargo Mate (cm) Integration ===
cargo() {
    cm exec "$@"
}
# Note: cm binary should be in PATH
alias cg='cm'
# === End Cargo Mate Integration ===
"#
        }
    };
    let mut file = OpenOptions::new().create(true).append(true).open(rc_file)?;
    writeln!(file, "{}", integration_code)?;
    println!("✅ Shell integration added to {}", rc_file.display());
    Ok(())
}
pub fn handle_activate() -> Result<()> {
    println!("🚀 Cargo Mate shell integration activated!");
    println!("   - Use \"cargo\" command to run cm exec");
    println!("   - Use \"cg\" as a shortcut for cm");
    println!("   - Restart your shell or run: source ~/.bashrc (or ~/.zshrc)");
    Ok(())
}