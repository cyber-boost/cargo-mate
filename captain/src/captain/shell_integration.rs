use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
pub struct ShellIntegration;
impl ShellIntegration {
    pub fn install() -> Result<()> {
        println!("Installing cargo-mate...");
        let shell = Self::detect_shell()?;
        let rc_file = Self::get_rc_file(&shell)?;
        println!("[SEARCH] Detected shell: {}", shell.cyan());
        println!("[FILE] RC file: {}", rc_file.display());
        Self::backup_rc_file(&rc_file)?;
        Self::ensure_shipwreck_bin_exists()?;
        let integration_code = Self::generate_integration_code(&shell);
        Self::add_to_rc_file(&rc_file, &integration_code)?;
        Self::create_completion_script(&shell)?;
        println!("[RELOAD] {}", "To activate immediately, run one of these:".yellow());
        println!("   {} {}", "source".green(), format!("{}", rc_file.display()) .cyan());
        println!("   {} {}", "cm".green(), "activate".cyan());
        println!("   {}", "Or restart your terminal".dimmed());
        println!();
        println!("[DOCS] {}", "Available commands after activation:".yellow());
        println!(" Run cm commands in cargo and run cargo commands in cm");
        println!("   {} - Direct cargo-mate access", "cm".cyan());
        println!();
        Ok(())
    }
    pub fn uninstall() -> Result<()> {
        println!("[TRASH] Removing cargo-mate shell integration...");
        let shell = Self::detect_shell()?;
        let rc_file = Self::get_rc_file(&shell)?;
        if !rc_file.exists() {
            println!("[WARN] RC file not found: {}", rc_file.display());
            return Ok(());
        }
        let content = fs::read_to_string(&rc_file)?;
        let cleaned = Self::remove_integration_code(&content);
        fs::write(&rc_file, cleaned)?;
        println!("[RELOAD] Please restart your terminal");
        Ok(())
    }
    fn ensure_shipwreck_bin_exists() -> Result<()> {
        let shipwreck_bin = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("bin");
        if !shipwreck_bin.exists() {
            fs::create_dir_all(&shipwreck_bin)?;
            println!("[DIR] Created directory: {}", shipwreck_bin.display());
        }
        let cm_binary = shipwreck_bin.join("cm");
        if cm_binary.exists() {
            println!("[OK] Found cm binary in {}", shipwreck_bin.display());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = std::fs::metadata(&cm_binary) {
                    let perms = metadata.permissions();
                    if perms.mode() & 0o111 == 0 {
                        println!("[WARN] cm binary is not executable, fixing...");
                        let current_mode = perms.mode();
                        let mut new_perms = perms;
                        new_perms.set_mode(current_mode | 0o755);
                        if let Err(e) = std::fs::set_permissions(&cm_binary, new_perms) {
                            println!("[ERROR] Failed to make cm executable: {}", e);
                        }
                    }
                }
            }
        } else {
            let mut found_binary = false;
            for entry in std::fs::read_dir(&shipwreck_bin)
                .unwrap_or_else(|_| std::fs::read_dir(".").unwrap())
            {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    if name_str.starts_with("cargo-mate")
                        && name_str.ends_with(".protected")
                    {
                        println!("[WARN] Found incorrectly named binary: {}", name_str);
                        println!(
                            "[FIX] This should be renamed to 'cm' for proper cargo integration"
                        );
                        found_binary = true;
                        break;
                    }
                }
            }
            if !found_binary {
                println!("[INFO] No cm binary found in {}", shipwreck_bin.display());
                println!(
                    "[INFO] The protected binary will be installed when you first run 'cm'"
                );
            }
        }
        Ok(())
    }
    pub fn detect_shell() -> Result<String> {
        let os_type = std::env::consts::OS;
        let is_windows = os_type == "windows";
        if is_windows {
            if std::env::var("PSModulePath").is_ok() {
                return Ok("powershell".to_string());
            } else {
                return Ok("cmd".to_string());
            }
        } else {
            if let Ok(shell_path) = std::env::var("SHELL") {
                let shell_name = std::path::Path::new(&shell_path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                match shell_name {
                    "zsh" => return Ok("zsh".to_string()),
                    "bash" => return Ok("bash".to_string()),
                    "fish" => return Ok("fish".to_string()),
                    "ash" | "dash" => return Ok("ash".to_string()),
                    "sh" => {
                        if os_type == "macos" {
                            return Ok("bash".to_string());
                        } else {
                            return Ok("ash".to_string());
                        }
                    }
                    _ => {
                        if shell_path.contains("zsh") {
                            return Ok("zsh".to_string());
                        } else if shell_path.contains("bash") {
                            return Ok("bash".to_string());
                        } else if shell_path.contains("fish") {
                            return Ok("fish".to_string());
                        }
                    }
                }
            }
            if os_type == "macos" {
                if std::process::Command::new("zsh").arg("--version").output().is_ok() {
                    return Ok("zsh".to_string());
                }
            }
        }
        match os_type {
            "macos" => Ok("zsh".to_string()),
            "linux" => Ok("bash".to_string()),
            _ => Ok("bash".to_string()),
        }
    }
    pub fn get_rc_file(shell: &str) -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let os_type = std::env::consts::OS;
        let is_windows = os_type == "windows";
        let is_macos = os_type == "macos";
        let rc_file = match shell {
            "powershell" => {
                if is_windows {
                    let profile_paths = vec![
                        home.join("Documents").join("PowerShell")
                        .join("Microsoft.PowerShell_profile.ps1"), home.join("Documents")
                        .join("WindowsPowerShell")
                        .join("Microsoft.PowerShell_profile.ps1"),
                    ];
                    for path in &profile_paths {
                        if path.exists() {
                            return Ok(path.clone());
                        }
                    }
                    profile_paths[0].clone()
                } else {
                    home.join(".profile")
                }
            }
            "cmd" => {
                if is_windows {
                    let autoexec = PathBuf::from("C:\\autoexec.bat");
                    if autoexec.exists() {
                        autoexec
                    } else {
                        home.join("cargo-mate-profile.cmd")
                    }
                } else {
                    home.join(".profile")
                }
            }
            "zsh" => {
                let zshrc = home.join(".zshrc");
                if zshrc.exists() || is_macos { zshrc } else { home.join(".profile") }
            }
            "bash" => {
                let bashrc = home.join(".bashrc");
                let bash_profile = home.join(".bash_profile");
                if bashrc.exists() {
                    bashrc
                } else if bash_profile.exists() {
                    bash_profile
                } else {
                    bashrc
                }
            }
            "fish" => {
                let config_dir = if let Ok(xdg_config) = std::env::var(
                    "XDG_CONFIG_HOME",
                ) {
                    PathBuf::from(xdg_config)
                } else {
                    home.join(".config")
                };
                let fish_config = config_dir.join("fish").join("config.fish");
                if let Some(parent) = fish_config.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                fish_config
            }
            "ash" => {
                let profile = home.join(".profile");
                if profile.exists() { profile } else { profile }
            }
            _ => {
                let profile = home.join(".profile");
                if profile.exists() { profile } else { profile }
            }
        };
        Ok(rc_file)
    }
    fn backup_rc_file(rc_file: &Path) -> Result<()> {
        if rc_file.exists() {
            let backup = rc_file.with_extension("bak.cargo-mate");
            fs::copy(rc_file, &backup)?;
            println!("[BACKUP] Backed up to: {}", backup.display());
        }
        Ok(())
    }
    fn generate_integration_code(shell: &str) -> String {
        let home = dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "$HOME".to_string());
        let shipwreck_bin = format!("{}/.shipwreck/bin", home);
        let os_type = std::env::consts::OS;
        let is_windows = os_type == "windows";
        match shell {
            "powershell" => {
                if is_windows {
                    r#"
# === Cargo Mate (cm) Integration for PowerShell ===
# This section was automatically added by cargo-mate

# Add Cargo Mate to PATH
$cmBinPath = "$env:USERPROFILE\.shipwreck\bin"
if ($env:PATH -notlike "*$cmBinPath*") {
    $env:PATH = "$cmBinPath;$env:PATH"
}

# Function to intercept cargo commands
function cargo {
    if (Get-Command cm -ErrorAction SilentlyContinue) {
        # Store original cargo path to avoid infinite loops
        $env:CARGO_BIN_PATH = (Get-Command cargo -ErrorAction SilentlyContinue).Source
        # Call cargo-mate instead
        & "$env:USERPROFILE\.shipwreck\bin\cm.exe" exec @args
    } else {
        # Fallback to original cargo if cm not found
        if ($env:CARGO_BIN_PATH) {
            & $env:CARGO_BIN_PATH @args
        } else {
            Write-Host "cargo command not found" -ForegroundColor Red
        }
    }
}

# Alias for quick access
Set-Alias cg cm

# Function to load project config
function cm_load_config {
    if (Test-Path .cg) {
        $env:CM_PROJECT_CONFIG = ".cg"
        # Uncomment below to show config loading
        # Write-Host "[ANCHOR] Loaded project config: .cg" -ForegroundColor Green
    }
}

# Load config when entering directory (PowerShell 6+)
if ($PSVersionTable.PSVersion.Major -ge 6) {
    Register-EngineEvent -SourceIdentifier PowerShell.OnIdle -Action {
        cm_load_config
    } | Out-Null
}

# Nautical theme
$env:CM_THEME = "nautical"

# === End Cargo Mate Integration ===
"#
                        .to_string()
                } else {
                    r#"
# PowerShell integration not supported on this platform
# Please use bash/zsh/fish integration instead
"#
                        .to_string()
                }
            }
            "cmd" => {
                if is_windows {
                    r#"
@echo off
REM === Cargo Mate (cm) Integration for CMD ===
REM This section was automatically added by cargo-mate

REM Add Cargo Mate to PATH
set CM_BIN_PATH=%USERPROFILE%\.shipwreck\bin
if "%PATH%"=="%PATH:%CM_BIN_PATH%=%" (
    set PATH=%CM_BIN_PATH%;%PATH%
)

REM Function to intercept cargo commands (via alias)
REM Note: CMD has limited function support, using alias instead
doskey cargo=cm exec $*

REM Alias for quick access
doskey cg=cm

REM Nautical theme
set CM_THEME=nautical

REM === End Cargo Mate Integration ===
"#
                        .to_string()
                } else {
                    r#"
# CMD integration not supported on this platform
# Please use bash/zsh/fish integration instead
"#
                        .to_string()
                }
            }
            "zsh" | "bash" => {
                r#"
# === Cargo Mate (cm) Integration ===
# This section was automatically added by cargo-mate

# Add ~/.shipwreck/bin to PATH for cargo-mate commands
if [[ ":$PATH:" != *":$HOME/.shipwreck/bin:"* ]]; then
    export PATH="$HOME/.shipwreck/bin:$PATH"
fi

# Function to check if cm command exists (more robust than command -v)
cm_exists() {
    # Try multiple ways to find cm
    if command -v cm &> /dev/null; then
        return 0
    elif [ -x "$HOME/.shipwreck/bin/cm" ]; then
        return 0
    elif [ -f "$HOME/.shipwreck/bin/cm" ]; then
        return 0
    fi
    return 1
}

# Function to intercept cargo commands
cargo() {
    if cm_exists; then
        # Set the path to the real cargo binary to avoid infinite loops
        export CARGO_BIN_PATH="$(command -v cargo 2>/dev/null || which cargo 2>/dev/null || echo 'cargo')"
        # Use the protected binary from .shipwreck/bin
        "$HOME/.shipwreck/bin/cm" exec "$@"
    else
        command cargo "$@"
    fi
}

# Alias for quick access
alias cg='cm'

# Auto-complete for cm
if [ -f ~/.shipwreck/completions/cm.bash ]; then
    source ~/.shipwreck/completions/cm.bash
fi

# Project-specific config loader
cm_load_config() {
    if [ -f .cg ]; then
        export CM_PROJECT_CONFIG=".cg"
        # Silent config loading (remove # to show once per session)
        # if [ -z "$CM_CONFIG_LOADED" ]; then
        #     echo "⚓ Loaded project config: .cg"
        #     export CM_CONFIG_LOADED="1"
        # fi
    fi
}

# Auto-load config when entering directory
if [[ "$SHELL" == *"zsh"* ]]; then
    autoload -U add-zsh-hook
    add-zsh-hook chpwd cm_load_config
elif [[ "$SHELL" == *"bash"* ]]; then
    PROMPT_COMMAND="${PROMPT_COMMAND:+$PROMPT_COMMAND;} cm_load_config"
fi

# Nautical prompt enhancement (optional)
export CM_THEME="nautical"

# === End Cargo Mate Integration ===
"#
                    .to_string()
            }
            "fish" => {
                r#"
# === Cargo Mate (cm) Integration ===
# This section was automatically added by cargo-mate

# Add ~/.shipwreck/bin to PATH for cargo-mate commands
if not contains $HOME/.shipwreck/bin $fish_user_paths
    set -U fish_user_paths $HOME/.shipwreck/bin $fish_user_paths
end

# Function to intercept cargo commands
function cargo
    if command -v cm > /dev/null
        # Set the path to the real cargo binary to avoid infinite loops
        set -x CARGO_BIN_PATH (command -v cargo)
        cm exec $argv
    else
        command cargo $argv
    end
end

# Aliases
alias cg='cm'

# Auto-complete
if test -f ~/.shipwreck/completions/cm.fish
    source ~/.shipwreck/completions/cm.fish
end

# Project config loader
function cm_load_config --on-variable PWD
    if test -f .cg
        set -x CM_PROJECT_CONFIG ".cg"
        echo "[ANCHOR] Loaded project config: .cg"
    end
end

# === End Cargo Mate Integration ===
"#
                    .to_string()
            }
            _ => String::new(),
        }
    }
    fn add_to_rc_file(rc_file: &Path, integration_code: &str) -> Result<()> {
        if let Some(parent) = rc_file.parent() {
            fs::create_dir_all(parent)
                .with_context(|| {
                    format!("Failed to create directory: {}", parent.display())
                })?;
        }
        let mut content = if rc_file.exists() {
            fs::read_to_string(rc_file)
                .with_context(|| {
                    format!("Failed to read RC file: {}", rc_file.display())
                })?
        } else {
            String::new()
        };
        let has_old_integration = content.contains("=== Cargo Mate");
        let has_new_integration = content.contains("Cargo Mate Integration");
        if has_old_integration || has_new_integration {
            println!("[WARN] Integration already exists, updating...");
            content = Self::remove_integration_code(&content);
        }
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        if !content.ends_with("\n\n") {
            content.push('\n');
        }
        content.push_str(integration_code);
        if !content.ends_with('\n') {
            content.push('\n');
        }
        Self::backup_rc_file(rc_file)?;
        fs::write(rc_file, &content)
            .with_context(|| format!("Failed to write RC file: {}", rc_file.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = fs::metadata(rc_file) {
                let mut perms = metadata.permissions();
                let current_mode = perms.mode();
                perms.set_mode(current_mode | 0o600);
                let _ = fs::set_permissions(rc_file, perms);
            }
        }
        Ok(())
    }
    fn remove_integration_code(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut in_section = false;
        for line in lines {
            if line.contains("=== Cargo Mate") && line.contains("Integration ===") {
                in_section = !in_section;
                continue;
            }
            if !in_section {
                result.push(line);
            }
        }
        result.join("\n")
    }
    fn create_completion_script(shell: &str) -> Result<()> {
        let completions_dir = dirs::home_dir()
            .context("Could not find home directory")?
            .join(".shipwreck")
            .join("completions");
        fs::create_dir_all(&completions_dir)?;
        let os_type = std::env::consts::OS;
        let is_windows = os_type == "windows";
        match shell {
            "powershell" => {
                if is_windows {
                    let script = r#"
# PowerShell completion for cm (cargo-mate)

using namespace System.Management.Automation
using namespace System.Management.Automation.Language

function CmCompletion {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commands = @(
        "build", "test", "run", "check", "clean", "doc", "fmt", "clippy",
        "init", "help", "journey", "anchor", "log", "tide", "map",
        "mutiny", "config", "version", "view", "optimize", "test",
        "checklist", "history", "install", "activate", "register", "idea", "wtf", "user"
    )

    $completions = $commands | Where-Object { $_ -like "$wordToComplete*" }

    foreach ($completion in $completions) {
        [CompletionResult]::new($completion, $completion, 'ParameterValue', $completion)
    }
}

Register-ArgumentCompleter -CommandName cm -ScriptBlock ${function:CmCompletion}
Register-ArgumentCompleter -CommandName cg -ScriptBlock ${function:CmCompletion}
"#;
                    let completion_file = completions_dir.join("cm.ps1");
                    fs::write(&completion_file, script)?;
                }
            }
            "cmd" => {
                if is_windows {
                    let script = r#"
REM CMD completion helper for cm (cargo-mate)
REM This file provides command information for manual completion

REM Available commands:
REM build test run check clean doc fmt clippy
REM init help journey anchor log tide map
REM mutiny config version view optimize test
REM checklist history install activate register idea wtf user
"#;
                    let completion_file = completions_dir.join("cm-help.cmd");
                    fs::write(&completion_file, script)?;
                }
            }
            "bash" | "zsh" => {
                let script = r#"#!/bin/bash
# Bash completion for cm (cargo-mate)

_cm_completions() {
    local cur prev opts
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    
    # Main commands
    opts="build test run check clean doc fmt clippy init help \
          journey anchor log tide map mutiny config checklist history"
    
    # Sub-commands
    case "${prev}" in
        journey)
            opts="record play list export import"
            ;;
        anchor)
            opts="save restore list show diff"
            ;;
        log)
            opts="add search timeline export analyze"
            ;;
        tide)
            opts="show analyze export"
            ;;
        map)
            opts="deps show analyze export"
            ;;
        mutiny)
            opts="activate deactivate allow-warnings skip-tests force yolo status"
            ;;
        config)
            opts="set get list init add-shortcut add-hook"
            ;;
        *)
            # Include cargo commands too
            if command -v cargo &> /dev/null; then
                cargo_opts=$(cargo --list 2>/dev/null | awk '{print $1}')
                opts="$opts $cargo_opts"
            fi
            ;;
    esac
    
    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
}

complete -F _cm_completions cm
complete -F _cm_completions cg
"#;
                let completion_file = completions_dir.join("cm.bash");
                fs::write(&completion_file, script)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&completion_file)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&completion_file, perms)?;
                }
            }
            "fish" => {
                let script = r#"# Fish completion for cm (cargo-mate)

complete -c cm -f

# Main commands
complete -c cm -n "__fish_use_subcommand" -a "build" -d "Compile the current package"
complete -c cm -n "__fish_use_subcommand" -a "test" -d "Run tests"
complete -c cm -n "__fish_use_subcommand" -a "run" -d "Run a binary"
complete -c cm -n "__fish_use_subcommand" -a "check" -d "Check code without building"
complete -c cm -n "__fish_use_subcommand" -a "journey" -d "Journey recording and playback"
complete -c cm -n "__fish_use_subcommand" -a "anchor" -d "Save and restore project states"
complete -c cm -n "__fish_use_subcommand" -a "log" -d "Captain's log"
complete -c cm -n "__fish_use_subcommand" -a "tide" -d "Performance tracking"
complete -c cm -n "__fish_use_subcommand" -a "map" -d "Dependency visualization"
complete -c cm -n "__fish_use_subcommand" -a "mutiny" -d "Override cargo restrictions"
complete -c cm -n "__fish_use_subcommand" -a "config" -d "Configuration management"

# Journey subcommands
complete -c cm -n "__fish_seen_subcommand_from journey" -a "record play list export import"

# Anchor subcommands
complete -c cm -n "__fish_seen_subcommand_from anchor" -a "save restore list show diff"

# Copy cg alias
complete -c cg -w cm
"#;
                let completion_file = completions_dir.join("cm.fish");
                fs::write(&completion_file, script)?;
            }
            _ => {}
        }
        Ok(())
    }
    pub fn show_status() {
        println!("{}", "=== Shell Integration Status ===".blue().bold());
        let shell = Self::detect_shell().unwrap_or_else(|_| "unknown".to_string());
        let os_type = std::env::consts::OS;
        let is_windows = os_type == "windows";
        println!("[SHELL] Current shell: {}", shell.cyan());
        println!("[OS] Operating system: {}", os_type.cyan());
        if let Ok(rc_file) = Self::get_rc_file(&shell) {
            if rc_file.exists() {
                let content = fs::read_to_string(&rc_file).unwrap_or_default();
                if content.contains("=== Cargo Mate")
                    || content.contains("Cargo Mate Integration")
                {
                    println!("[OK] Integration: {}", "Installed".green());
                    println!(
                        "   Config file: {}", rc_file.display().to_string().dimmed()
                    );
                } else {
                    println!("[X] Integration: {}", "Not installed".red());
                }
            } else {
                println!("[X] Integration: {}", "Not installed".red());
                println!(
                    "   Config file location: {}", rc_file.display().to_string().dimmed()
                );
            }
        }
        if let Ok(path) = std::env::var("PATH") {
            let separator = if is_windows { ';' } else { ':' };
            let cm_in_path = path
                .split(separator)
                .any(|p| {
                    let cm_path = Path::new(p)
                        .join(if is_windows { "cm.exe" } else { "cm" });
                    cm_path.exists()
                });
            if cm_in_path {
                println!("[OK] Binary in PATH: {}", "Yes".green());
            } else {
                println!("[WARN] Binary in PATH: {}", "No".yellow());
            }
        }
        let completions_dir = dirs::home_dir()
            .map(|h| h.join(".shipwreck").join("completions"))
            .unwrap_or_default();
        if completions_dir.exists() {
            println!("[OK] Completions: {}", "Installed".green());
            let extensions = if is_windows {
                vec!["ps1", "cmd"]
            } else {
                vec!["bash", "fish"]
            };
            for ext in extensions {
                let completion_file = completions_dir.join(format!("cm.{}", ext));
                if completion_file.exists() {
                    println!(
                        "   {} {} completion", "•".dimmed(), ext.to_uppercase()
                        .dimmed()
                    );
                }
            }
        } else {
            println!("[X] Completions: {}", "Not installed".red());
        }
        if is_windows {
            println!();
            println!("[WINDOWS] Windows-specific notes:");
            println!("   • Use PowerShell for best experience");
            println!("   • CMD support is limited");
            println!("   • Admin privileges may be required for installation");
        }
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
}